use anyhow::{bail, ensure, Context, Result};
use std::path::PathBuf;
use tracing;

pub struct MetricAggregator;

impl MetricAggregator {
    /// Generate leaderboard and aggregated metrics CSV files from benchmark directory
    pub fn generate_csv_from_benchmark_dir(benchmark_dir: &PathBuf) -> Result<()> {
        use std::process::Command;

        // Step 1: Run prepare_aggregate_metrics_with_errors.py to create aggregate_metrics.csv files
        let prepare_script_path = std::env::current_dir()
            .context("Failed to get current working directory")?
            .join("scripts")
            .join("bench-postprocess-scripts")
            .join("prepare_aggregate_metrics_with_errors.py");

        ensure!(
            prepare_script_path.exists(),
            "Prepare script not found: {}",
            prepare_script_path.display()
        );

        tracing::info!(
            "Preparing aggregate metrics from benchmark directory: {}",
            benchmark_dir.display()
        );

        let output = Command::new(&prepare_script_path)
            .arg("--benchmark-dir")
            .arg(benchmark_dir)
            .output()
            .context("Failed to execute prepare_aggregate_metrics_with_errors.py script")?;

        if !output.status.success() {
            let error_message = String::from_utf8_lossy(&output.stderr);
            bail!("Failed to prepare aggregate metrics: {}", error_message);
        }

        let success_message = String::from_utf8_lossy(&output.stdout);
        tracing::info!("{}", success_message);

        // Step 2: Run generate_leaderboard_with_errors.py to create the final leaderboard
        let leaderboard_script_path = std::env::current_dir()
            .context("Failed to get current working directory")?
            .join("scripts")
            .join("bench-postprocess-scripts")
            .join("generate_leaderboard_with_errors.py");

        if leaderboard_script_path.exists() {
            tracing::info!(
                "Generating leaderboard from benchmark directory: {}",
                benchmark_dir.display()
            );

            let output = Command::new(&leaderboard_script_path)
                .arg("--benchmark-dir")
                .arg(benchmark_dir)
                .arg("--leaderboard-output")
                .arg("leaderboard.csv")
                .arg("--union-output")
                .arg("all_metrics.csv")
                .output()
                .context("Failed to execute generate_leaderboard_with_errors.py script")?;

            if !output.status.success() {
                let error_message = String::from_utf8_lossy(&output.stderr);
                bail!("Failed to generate leaderboard: {}", error_message);
            }

            let success_message = String::from_utf8_lossy(&output.stdout);
            tracing::info!("{}", success_message);
        } else {
            // Fallback to aggregate_benchmark_results.py if generate_leaderboard_with_errors.py doesn't exist
            let aggregate_script_path = std::env::current_dir()
                .context("Failed to get current working directory")?
                .join("scripts")
                .join("bench-postprocess-scripts")
                .join("aggregate_benchmark_results.py");

            if aggregate_script_path.exists() {
                tracing::info!("Using fallback aggregation script");

                let output = Command::new(&aggregate_script_path)
                    .arg(benchmark_dir)
                    .arg("--output")
                    .arg("aggregated_benchmark_results.csv")
                    .output()
                    .context("Failed to execute aggregate_benchmark_results.py script")?;

                if !output.status.success() {
                    let error_message = String::from_utf8_lossy(&output.stderr);
                    bail!("Failed to aggregate benchmark results: {}", error_message);
                }

                let success_message = String::from_utf8_lossy(&output.stdout);
                tracing::info!("{}", success_message);
            }
        }

        Ok(())
    }
}
