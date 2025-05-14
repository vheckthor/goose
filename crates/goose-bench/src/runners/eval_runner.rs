use crate::bench_config::{BenchEval, BenchModel, BenchRunConfig};
use crate::bench_session::BenchAgent;
use crate::bench_work_dir::BenchmarkWorkDir;
use crate::errors::{BenchError, BenchResult};
use crate::eval_suites::{EvaluationSuite, ExtensionRequirements};
use crate::logging;
use crate::reporting::EvaluationResult;
use crate::utilities::await_process_exits;
use std::env;
use std::fs;
use std::future::Future;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone)]
pub struct EvalRunner {
    config: BenchRunConfig,
}

impl EvalRunner {
    pub fn from(config: String) -> BenchResult<EvalRunner> {
        let config = BenchRunConfig::from_string(config)
            .map_err(|e| BenchError::ConfigError(format!("Failed to parse config: {}", e)))?;
        Ok(EvalRunner { config })
    }

    fn create_work_dir(&self, config: &BenchRunConfig) -> BenchResult<BenchmarkWorkDir> {
        let goose_model = config
            .models
            .first()
            .ok_or_else(|| BenchError::ConfigError("No model specified in config".to_string()))?;
        let model_name = goose_model.name.clone();
        let provider_name = goose_model.provider.clone();

        // construct work-dir name to have a shim component only if shim configured to be used
        let work_dir_name_shim = {
            let mut shim_name = "".to_string();
            if let Some(shim_opt) = &goose_model.tool_shim {
                if shim_opt.use_tool_shim {
                    let shim_model = if let Some(shim_model) = &shim_opt.tool_shim_model {
                        shim_model.clone()
                    } else {
                        "default".to_string()
                    };
                    shim_name = format!("-{}-shim-model", shim_model);
                }
            }
            shim_name
        };

        let include_dir = config.include_dirs.clone();
        let work_dir_name = format!("{}-{}{}", provider_name, model_name, work_dir_name_shim);
        let work_dir = BenchmarkWorkDir::new(work_dir_name, include_dir);
        Ok(work_dir)
    }

    pub async fn run<F, Fut>(&mut self, agent_generator: F) -> BenchResult<()>
    where
        F: Fn(ExtensionRequirements, String) -> Fut,
        Fut: Future<Output = BenchAgent> + Send,
    {
        let mut work_dir = self.create_work_dir(&self.config).map_err(|e| {
            BenchError::BenchmarkError(format!("Failed to create work directory: {}", e))
        })?;

        let bench_eval = self.config.evals.first().ok_or_else(|| {
            BenchError::ConfigError("No evaluations specified in config".to_string())
        })?;

        let run_id = &self
            .config
            .run_id
            .clone()
            .unwrap_or_else(|| "run-0".to_string());
        let run_id = format!("run-{}", run_id.clone());

        // create entire dir subtree for eval and cd into dir for running eval
        work_dir.set_eval(&bench_eval.selector, run_id);
        logging::info(&format!(
            "Set evaluation directory for {}",
            bench_eval.selector
        ));

        if let Some(eval) = EvaluationSuite::from(&bench_eval.selector) {
            let now_stamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_err(|e| BenchError::Other(format!("Failed to get timestamp: {}", e)))?
                .as_nanos();

            let session_id = format!("{}-{}", bench_eval.selector.clone(), now_stamp);
            logging::info(&format!("Created session ID: {}", session_id));

            let mut agent = agent_generator(eval.required_extensions(), session_id).await;
            logging::info(&format!("Agent created for {}", eval.name()));

            let mut result = EvaluationResult::new(eval.name().to_string());

            match eval.run(&mut agent, &mut work_dir).await {
                Ok(metrics) => {
                    logging::info(&format!(
                        "Evaluation run successful with {} metrics",
                        metrics.len()
                    ));
                    for (name, metric) in metrics {
                        result.add_metric(name, metric);
                    }
                }
                Err(e) => {
                    logging::error(&format!("Evaluation run failed: {}", e));
                }
            }

            // Add any errors that occurred
            let errors = agent.get_errors().await;
            logging::info(&format!("Agent reported {} errors", errors.len()));
            for error in errors {
                result.add_error(error);
            }

            // Write results to file
            let eval_results =
                serde_json::to_string_pretty(&result).map_err(|e| BenchError::JsonParseError(e))?;

            let eval_results_file = env::current_dir()
                .map_err(|e| BenchError::IoError(e))?
                .join(&self.config.eval_result_filename);

            fs::write(&eval_results_file, &eval_results).map_err(|e| BenchError::IoError(e))?;

            logging::info(&format!(
                "Wrote evaluation results to {}",
                eval_results_file.display()
            ));

            self.config.save("config.cfg".to_string());
            work_dir.save();

            // handle running post-process cmd if configured
            if let Some(cmd) = &bench_eval.post_process_cmd {
                logging::info(&format!("Running post-process command: {:?}", cmd));

                let handle = Command::new(cmd)
                    .arg(&eval_results_file)
                    .spawn()
                    .map_err(|e| BenchError::IoError(e))?;

                await_process_exits(&mut [handle], Vec::new());
            }

            // copy session file into eval-dir
            let here = env::current_dir()
                .map_err(|e| BenchError::IoError(e))?
                .canonicalize()
                .map_err(|e| BenchError::IoError(e))?;

            BenchmarkWorkDir::deep_copy(agent.session_file().as_path(), here.as_path(), false)
                .map_err(|e| BenchError::IoError(e))?;

            logging::info("Evaluation completed successfully");
        } else {
            logging::error(&format!(
                "No evaluation found for selector: {}",
                bench_eval.selector
            ));
            return Err(BenchError::EvaluationError(format!(
                "No evaluation found for selector: {}",
                bench_eval.selector
            )));
        }

        Ok(())
    }

    pub fn path_for_eval(model: &BenchModel, eval: &BenchEval, run_id: String) -> PathBuf {
        let provider = model.provider.clone();
        let model = model.name.clone();
        let eval_path = &eval.selector.replace(":", std::path::MAIN_SEPARATOR_STR);
        let eval_results_location = format!(
            "{}-{}/run-{}{}{}",
            &provider,
            model,
            run_id,
            std::path::MAIN_SEPARATOR_STR,
            eval_path
        );
        PathBuf::from(eval_results_location.clone())
    }
}
