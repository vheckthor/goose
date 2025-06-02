use std::process::Command;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tokio::time::sleep;
use tracing::{info, warn};

use crate::scheduler::{ScheduledJob, SchedulerError};
use crate::scheduler_trait::SchedulerTrait;
use crate::session::storage::SessionMetadata;

const TEMPORAL_SERVICE_URL: &str = "http://localhost:8080";
const TEMPORAL_SERVER_URL: &str = "http://localhost:7233";
const TEMPORAL_SERVICE_STARTUP_TIMEOUT: Duration = Duration::from_secs(30);
const TEMPORAL_SERVICE_HEALTH_CHECK_INTERVAL: Duration = Duration::from_secs(2);

#[derive(Serialize, Deserialize, Debug)]
struct JobRequest {
    action: String,
    job_id: Option<String>,
    cron: Option<String>,
    recipe_path: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
struct JobResponse {
    success: bool,
    message: String,
    jobs: Option<Vec<TemporalJobStatus>>,
    data: Option<serde_json::Value>,
}

#[derive(Serialize, Deserialize, Debug)]
struct TemporalJobStatus {
    id: String,
    cron: String,
    recipe_path: String,
    last_run: Option<String>,
    next_run: Option<String>,
    currently_running: bool,
    paused: bool,
    created_at: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct RunNowResponse {
    session_id: String,
}

pub struct TemporalScheduler {
    http_client: Client,
    service_url: String,
}

impl TemporalScheduler {
    pub async fn new() -> Result<Arc<Self>, SchedulerError> {
        let http_client = Client::new();
        let service_url = TEMPORAL_SERVICE_URL.to_string();

        let scheduler = Arc::new(Self {
            http_client,
            service_url,
        });

        // Check if services are running, start them if needed
        scheduler.ensure_services_running().await?;

        // Wait for service to be ready
        scheduler.wait_for_service_ready().await?;

        info!("TemporalScheduler initialized successfully");
        Ok(scheduler)
    }

    async fn ensure_services_running(&self) -> Result<(), SchedulerError> {
        info!("Checking if Temporal services are running...");

        // Check if Go service is already running
        if self.health_check().await? {
            info!("Temporal Go service is already running");
            return Ok(());
        }

        // Check if Temporal server is running
        let temporal_running = self.check_temporal_server().await;

        if !temporal_running {
            info!("Starting Temporal server...");
            self.start_temporal_server().await?;

            // Wait for Temporal server to be ready
            self.wait_for_temporal_server().await?;
        } else {
            info!("Temporal server is already running");
        }

        // Check Go service again
        if !self.health_check().await? {
            info!("Starting Temporal Go service...");
            self.start_go_service().await?;
        }

        Ok(())
    }

    async fn check_temporal_server(&self) -> bool {
        match self
            .http_client
            .get(format!("{}/api/v1/namespaces", TEMPORAL_SERVER_URL))
            .send()
            .await
        {
            Ok(response) => response.status().is_success(),
            Err(_) => false,
        }
    }

    async fn start_temporal_server(&self) -> Result<(), SchedulerError> {
        info!("Starting Temporal server in background...");

        let output = Command::new("sh")
            .arg("-c")
            .arg("nohup temporal server start-dev --db-filename temporal.db --port 7233 --ui-port 8233 --log-level warn > temporal-server.log 2>&1 & echo $!")
            .output()
            .map_err(|e| SchedulerError::SchedulerInternalError(
                format!("Failed to start Temporal server: {}. Make sure 'temporal' CLI is installed.", e)
            ))?;

        if !output.status.success() {
            return Err(SchedulerError::SchedulerInternalError(format!(
                "Failed to start Temporal server: {}",
                String::from_utf8_lossy(&output.stderr)
            )));
        }

        let pid_output = String::from_utf8_lossy(&output.stdout);
        let pid = pid_output.trim();
        info!("Temporal server started with PID: {}", pid);

        Ok(())
    }

    async fn wait_for_temporal_server(&self) -> Result<(), SchedulerError> {
        info!("Waiting for Temporal server to be ready...");

        let start_time = std::time::Instant::now();

        while start_time.elapsed() < TEMPORAL_SERVICE_STARTUP_TIMEOUT {
            if self.check_temporal_server().await {
                info!("Temporal server is ready");
                return Ok(());
            }
            sleep(TEMPORAL_SERVICE_HEALTH_CHECK_INTERVAL).await;
        }

        Err(SchedulerError::SchedulerInternalError(
            "Temporal server failed to become ready within timeout".to_string(),
        ))
    }

    async fn start_go_service(&self) -> Result<(), SchedulerError> {
        info!("Starting Temporal Go service in background...");

        // Check if the temporal-service binary exists
        let binary_path = "./temporal-service/temporal-service";
        if !std::path::Path::new(binary_path).exists() {
            return Err(SchedulerError::SchedulerInternalError(
                format!("Go service binary not found at {}. Please build it with './temporal-service/build.sh'", binary_path)
            ));
        }

        let output = Command::new("sh")
            .arg("-c")
            .arg("cd temporal-service && nohup ./temporal-service > temporal-service.log 2>&1 & echo $!")
            .output()
            .map_err(|e| SchedulerError::SchedulerInternalError(
                format!("Failed to start Go temporal service: {}", e)
            ))?;

        if !output.status.success() {
            return Err(SchedulerError::SchedulerInternalError(format!(
                "Failed to start Go service: {}",
                String::from_utf8_lossy(&output.stderr)
            )));
        }

        let pid_output = String::from_utf8_lossy(&output.stdout);
        let pid = pid_output.trim();
        info!("Temporal Go service started with PID: {}", pid);

        Ok(())
    }

    async fn wait_for_service_ready(&self) -> Result<(), SchedulerError> {
        info!("Waiting for Temporal service to be ready...");

        let start_time = std::time::Instant::now();

        while start_time.elapsed() < TEMPORAL_SERVICE_STARTUP_TIMEOUT {
            match self.health_check().await {
                Ok(true) => {
                    info!("Temporal service is ready");
                    return Ok(());
                }
                Ok(false) => {
                    // Service responded but not healthy
                    sleep(TEMPORAL_SERVICE_HEALTH_CHECK_INTERVAL).await;
                }
                Err(_) => {
                    // Service not responding yet
                    sleep(TEMPORAL_SERVICE_HEALTH_CHECK_INTERVAL).await;
                }
            }
        }

        Err(SchedulerError::SchedulerInternalError(
            "Temporal service failed to become ready within timeout".to_string(),
        ))
    }

    async fn health_check(&self) -> Result<bool, SchedulerError> {
        let url = format!("{}/health", self.service_url);

        match self.http_client.get(&url).send().await {
            Ok(response) => {
                if response.status().is_success() {
                    Ok(true)
                } else {
                    Ok(false)
                }
            }
            Err(_) => Ok(false),
        }
    }

    pub async fn add_scheduled_job(&self, job: ScheduledJob) -> Result<(), SchedulerError> {
        let request = JobRequest {
            action: "create".to_string(),
            job_id: Some(job.id.clone()),
            cron: Some(job.cron.clone()),
            recipe_path: Some(job.source.clone()),
        };

        let response = self.make_request(request).await?;

        if response.success {
            info!("Successfully created scheduled job: {}", job.id);
            Ok(())
        } else {
            Err(SchedulerError::SchedulerInternalError(response.message))
        }
    }

    pub async fn list_scheduled_jobs(&self) -> Result<Vec<ScheduledJob>, SchedulerError> {
        let request = JobRequest {
            action: "list".to_string(),
            job_id: None,
            cron: None,
            recipe_path: None,
        };

        let response = self.make_request(request).await?;

        if response.success {
            let jobs = response.jobs.unwrap_or_default();
            let scheduled_jobs = jobs
                .into_iter()
                .map(|tj| {
                    ScheduledJob {
                        id: tj.id,
                        source: tj.recipe_path,
                        cron: tj.cron,
                        last_run: tj.last_run.and_then(|s| s.parse::<DateTime<Utc>>().ok()),
                        currently_running: tj.currently_running,
                        paused: tj.paused,
                        current_session_id: None, // Not provided by Temporal service
                        process_start_time: None, // Not provided by Temporal service
                    }
                })
                .collect();
            Ok(scheduled_jobs)
        } else {
            Err(SchedulerError::SchedulerInternalError(response.message))
        }
    }

    pub async fn remove_scheduled_job(&self, id: &str) -> Result<(), SchedulerError> {
        let request = JobRequest {
            action: "delete".to_string(),
            job_id: Some(id.to_string()),
            cron: None,
            recipe_path: None,
        };

        let response = self.make_request(request).await?;

        if response.success {
            info!("Successfully removed scheduled job: {}", id);
            Ok(())
        } else {
            Err(SchedulerError::SchedulerInternalError(response.message))
        }
    }

    pub async fn pause_schedule(&self, id: &str) -> Result<(), SchedulerError> {
        let request = JobRequest {
            action: "pause".to_string(),
            job_id: Some(id.to_string()),
            cron: None,
            recipe_path: None,
        };

        let response = self.make_request(request).await?;

        if response.success {
            info!("Successfully paused scheduled job: {}", id);
            Ok(())
        } else {
            Err(SchedulerError::SchedulerInternalError(response.message))
        }
    }

    pub async fn unpause_schedule(&self, id: &str) -> Result<(), SchedulerError> {
        let request = JobRequest {
            action: "unpause".to_string(),
            job_id: Some(id.to_string()),
            cron: None,
            recipe_path: None,
        };

        let response = self.make_request(request).await?;

        if response.success {
            info!("Successfully unpaused scheduled job: {}", id);
            Ok(())
        } else {
            Err(SchedulerError::SchedulerInternalError(response.message))
        }
    }

    pub async fn run_now(&self, id: &str) -> Result<String, SchedulerError> {
        let request = JobRequest {
            action: "run_now".to_string(),
            job_id: Some(id.to_string()),
            cron: None,
            recipe_path: None,
        };

        let response = self.make_request(request).await?;

        if response.success {
            if let Some(data) = response.data {
                if let Ok(run_response) = serde_json::from_value::<RunNowResponse>(data) {
                    info!("Successfully started job execution for: {}", id);
                    Ok(run_response.session_id)
                } else {
                    Err(SchedulerError::SchedulerInternalError(
                        "Invalid response format for run_now".to_string(),
                    ))
                }
            } else {
                Err(SchedulerError::SchedulerInternalError(
                    "No session ID returned from run_now".to_string(),
                ))
            }
        } else {
            Err(SchedulerError::SchedulerInternalError(response.message))
        }
    }

    // Note: These methods are not directly supported by the Temporal service
    // but are kept for API compatibility
    pub async fn sessions(
        &self,
        _sched_id: &str,
        _limit: usize,
    ) -> Result<Vec<(String, SessionMetadata)>, SchedulerError> {
        warn!("sessions() method not implemented for TemporalScheduler - use session storage directly");
        Ok(vec![])
    }

    pub async fn update_schedule(
        &self,
        _sched_id: &str,
        _new_cron: String,
    ) -> Result<(), SchedulerError> {
        warn!("update_schedule() method not implemented for TemporalScheduler - delete and recreate job instead");
        Err(SchedulerError::SchedulerInternalError(
            "update_schedule not supported - delete and recreate job instead".to_string(),
        ))
    }

    pub async fn kill_running_job(&self, _sched_id: &str) -> Result<(), SchedulerError> {
        warn!("kill_running_job() method not implemented for TemporalScheduler");
        Err(SchedulerError::SchedulerInternalError(
            "kill_running_job not supported by TemporalScheduler".to_string(),
        ))
    }

    pub async fn get_running_job_info(
        &self,
        _sched_id: &str,
    ) -> Result<Option<(String, DateTime<Utc>)>, SchedulerError> {
        warn!("get_running_job_info() method not implemented for TemporalScheduler");
        Ok(None)
    }

    async fn make_request(&self, request: JobRequest) -> Result<JobResponse, SchedulerError> {
        let url = format!("{}/jobs", self.service_url);

        let response = self
            .http_client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| {
                SchedulerError::SchedulerInternalError(format!("HTTP request failed: {}", e))
            })?;

        if !response.status().is_success() {
            return Err(SchedulerError::SchedulerInternalError(format!(
                "HTTP request failed with status: {}",
                response.status()
            )));
        }

        let job_response: JobResponse = response.json().await.map_err(|e| {
            SchedulerError::SchedulerInternalError(format!("Failed to parse response JSON: {}", e))
        })?;

        Ok(job_response)
    }
}

impl Drop for TemporalScheduler {
    fn drop(&mut self) {
        // Services continue running independently - no cleanup needed
        info!("TemporalScheduler dropped - Temporal services continue running independently");
    }
}

// Service management utilities
impl TemporalScheduler {
    /// Check if Temporal services are running
    pub async fn check_services_status(&self) -> (bool, bool) {
        let temporal_server_running = self.check_temporal_server().await;
        let go_service_running = self.health_check().await.unwrap_or(false);
        (temporal_server_running, go_service_running)
    }

    /// Get service information
    pub async fn get_service_info(&self) -> String {
        let (temporal_running, go_running) = self.check_services_status().await;

        format!(
            "Temporal Services Status:\n\
             - Temporal Server ({}:7233): {}\n\
             - Temporal Web UI: http://localhost:8233\n\
             - Go Service ({}:8080): {}\n\
             - Service logs: temporal-server.log, temporal-service/temporal-service.log",
            if temporal_running {
                "localhost"
            } else {
                "not running"
            },
            if temporal_running {
                "✅ Running"
            } else {
                "❌ Not Running"
            },
            if go_running {
                "localhost"
            } else {
                "not running"
            },
            if go_running {
                "✅ Running"
            } else {
                "❌ Not Running"
            }
        )
    }

    /// Stop Temporal services (for manual management)
    pub async fn stop_services(&self) -> Result<String, SchedulerError> {
        info!("Stopping Temporal services...");

        let mut results = Vec::new();

        // Stop Go service
        let go_result = Command::new("pkill")
            .args(["-f", "temporal-service"])
            .output();

        match go_result {
            Ok(output) if output.status.success() => {
                results.push("✅ Go service stopped".to_string());
            }
            Ok(_) => {
                results.push("⚠️  Go service was not running or failed to stop".to_string());
            }
            Err(e) => {
                results.push(format!("❌ Failed to stop Go service: {}", e));
            }
        }

        // Stop Temporal server
        let temporal_result = Command::new("pkill")
            .args(["-f", "temporal server start-dev"])
            .output();

        match temporal_result {
            Ok(output) if output.status.success() => {
                results.push("✅ Temporal server stopped".to_string());
            }
            Ok(_) => {
                results.push("⚠️  Temporal server was not running or failed to stop".to_string());
            }
            Err(e) => {
                results.push(format!("❌ Failed to stop Temporal server: {}", e));
            }
        }

        let result_message = results.join("\n");
        info!("Service stop results: {}", result_message);
        Ok(result_message)
    }
}

#[async_trait]
impl SchedulerTrait for TemporalScheduler {
    async fn add_scheduled_job(&self, job: ScheduledJob) -> Result<(), SchedulerError> {
        self.add_scheduled_job(job).await
    }

    async fn list_scheduled_jobs(&self) -> Result<Vec<ScheduledJob>, SchedulerError> {
        self.list_scheduled_jobs().await
    }

    async fn remove_scheduled_job(&self, id: &str) -> Result<(), SchedulerError> {
        self.remove_scheduled_job(id).await
    }

    async fn pause_schedule(&self, id: &str) -> Result<(), SchedulerError> {
        self.pause_schedule(id).await
    }

    async fn unpause_schedule(&self, id: &str) -> Result<(), SchedulerError> {
        self.unpause_schedule(id).await
    }

    async fn run_now(&self, id: &str) -> Result<String, SchedulerError> {
        self.run_now(id).await
    }

    async fn sessions(
        &self,
        sched_id: &str,
        limit: usize,
    ) -> Result<Vec<(String, SessionMetadata)>, SchedulerError> {
        self.sessions(sched_id, limit).await
    }

    async fn update_schedule(
        &self,
        sched_id: &str,
        new_cron: String,
    ) -> Result<(), SchedulerError> {
        self.update_schedule(sched_id, new_cron).await
    }

    async fn kill_running_job(&self, sched_id: &str) -> Result<(), SchedulerError> {
        self.kill_running_job(sched_id).await
    }

    async fn get_running_job_info(
        &self,
        sched_id: &str,
    ) -> Result<Option<(String, DateTime<Utc>)>, SchedulerError> {
        self.get_running_job_info(sched_id).await
    }
}
