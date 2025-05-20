use crate::bench_config::{BenchEval, BenchModel, BenchRunConfig};
use crate::errors::{BenchError, BenchResult};
use crate::eval_suites::EvaluationSuite;
use crate::reporting::{BenchmarkResults, SuiteResult};
use crate::runners::eval_runner::EvalRunner;
use crate::utilities::{await_process_exits, parallel_bench_cmd};
use dotenvy::from_path_iter;
use std::collections::HashMap;
use std::fs::read_to_string;
use std::path::PathBuf;
use std::process::Child;
use std::thread;
use tracing;

#[derive(Clone)]
pub struct ModelRunner {
    config: BenchRunConfig,
}

impl ModelRunner {
    pub fn from(config: String) -> BenchResult<ModelRunner> {
        let config = BenchRunConfig::from_string(config)
            .map_err(|e| BenchError::ConfigError(format!("Failed to parse config: {}", e)))?;
        Ok(ModelRunner { config })
    }

    /// Generate leaderboard and metrics CSV files from benchmark directory
    pub fn generate_csv_from_benchmark_dir(benchmark_dir: &PathBuf) -> BenchResult<()> {
        let script_path = std::env::current_dir()
            .map_err(|e| BenchError::IoError(e))?
            .join("scripts")
            .join("bench-postprocess-scripts")
            .join("generate_leaderboard.py");

        if !script_path.exists() {
            return Err(BenchError::FileNotFound(script_path));
        }

        use std::process::Command;

        tracing::info!(
            "Generating leaderboard from benchmark directory: {}",
            benchmark_dir.display()
        );

        let output = Command::new(&script_path)
            .arg("--benchmark-dir")
            .arg(benchmark_dir)
            .arg("--leaderboard-output")
            .arg("leaderboard.csv")
            .arg("--union-output")
            .arg("all_metrics.csv")
            .output()
            .map_err(|e| BenchError::IoError(e))?;

        if !output.status.success() {
            let error_message = String::from_utf8_lossy(&output.stderr);
            return Err(BenchError::BenchmarkError(format!(
                "Failed to generate leaderboard: {}",
                error_message
            )));
        }

        let success_message = String::from_utf8_lossy(&output.stdout);
        tracing::info!("{}", success_message);

        Ok(())
    }

    pub fn run(&self) -> BenchResult<()> {
        let model =
            self.config.models.first().ok_or_else(|| {
                BenchError::ConfigError("No model specified in config".to_string())
            })?;
        let suites = self.collect_evals_for_run();

        let mut handles = vec![];

        for i in 0..self.config.repeat.unwrap_or(1) {
            let self_copy = self.clone();
            let model_clone = model.clone();
            let suites_clone = suites.clone();
            let handle = thread::spawn(move || -> BenchResult<()> {
                self_copy.run_benchmark(&model_clone, suites_clone, i.to_string())
            });
            handles.push(handle);
        }
        await_process_exits(&mut Vec::new(), handles);

        let mut all_runs_results: Vec<BenchmarkResults> = Vec::new();
        for i in 0..self.config.repeat.unwrap_or(1) {
            match self.collect_run_results(model.clone(), suites.clone(), i.to_string()) {
                Ok(run_results) => all_runs_results.push(run_results),
                Err(e) => {
                    tracing::error!("Failed to collect results for run {}: {}", i, e)
                }
            }
        }

        Ok(())
    }

    fn run_benchmark(
        &self,
        model: &BenchModel,
        suites: HashMap<String, Vec<BenchEval>>,
        run_id: String,
    ) -> BenchResult<()> {
        let mut results_handles = HashMap::<String, Vec<Child>>::new();

        // Load environment variables from file if specified
        let mut envs = self.toolshim_envs();
        if let Some(env_file) = &self.config.env_file {
            let env_vars = ModelRunner::load_env_file(env_file).map_err(|e| {
                BenchError::EnvironmentError(format!("Failed to load environment file: {}", e))
            })?;
            envs.extend(env_vars);
        }
        envs.push(("GOOSE_MODEL".to_string(), model.clone().name));
        envs.push(("GOOSE_PROVIDER".to_string(), model.clone().provider));

        // Only run in parallel if the model is parallel_safe
        let run_parallel = model.parallel_safe;

        for (suite, evals) in suites.iter() {
            results_handles.insert((*suite).clone(), Vec::new());

            // Group evaluations by parallel_safe
            let mut parallel_evals = Vec::new();
            let mut sequential_evals = Vec::new();

            for eval in evals {
                if eval.parallel_safe && run_parallel {
                    parallel_evals.push(eval);
                } else {
                    sequential_evals.push(eval);
                }
            }

            // Run parallel-safe evaluations in parallel
            if !parallel_evals.is_empty() {
                for eval_selector in &parallel_evals {
                    let mut config_copy = self.config.clone();
                    config_copy.run_id = Some(run_id.clone());
                    config_copy.evals = vec![(*eval_selector).clone()];
                    let cfg = config_copy.to_string().map_err(|e| {
                        BenchError::ConfigError(format!("Failed to serialize config: {}", e))
                    })?;

                    let handle = parallel_bench_cmd("exec-eval".to_string(), cfg, envs.clone());
                    results_handles.get_mut(suite).unwrap().push(handle);
                }
            }

            // Run non-parallel-safe evaluations sequentially
            for eval_selector in &sequential_evals {
                let mut config_copy = self.config.clone();
                config_copy.run_id = Some(run_id.clone());
                config_copy.evals = vec![(*eval_selector).clone()];
                let cfg = config_copy.to_string().map_err(|e| {
                    BenchError::ConfigError(format!("Failed to serialize config: {}", e))
                })?;

                let handle = parallel_bench_cmd("exec-eval".to_string(), cfg, envs.clone());

                // Wait for this process to complete before starting the next one
                let mut child_procs = vec![handle];
                await_process_exits(&mut child_procs, Vec::new());
            }
        }

        // Wait for any remaining parallel processes to complete
        for (_, child_procs) in results_handles.iter_mut() {
            await_process_exits(child_procs, Vec::new());
        }

        Ok(())
    }

    fn collect_run_results(
        &self,
        model: BenchModel,
        suites: HashMap<String, Vec<BenchEval>>,
        run_id: String,
    ) -> BenchResult<BenchmarkResults> {
        let mut results = BenchmarkResults::new(model.provider.clone());

        let mut summary_path: Option<PathBuf> = None;

        for (suite, evals) in suites.iter() {
            let mut suite_result = SuiteResult::new(suite.clone());
            for eval_selector in evals {
                let mut eval_path =
                    EvalRunner::path_for_eval(&model, eval_selector, run_id.clone());
                eval_path.push(self.config.eval_result_filename.clone());

                let content = read_to_string(&eval_path).map_err(|e| BenchError::IoError(e))?;

                let eval_result =
                    serde_json::from_str(&content).map_err(|e| BenchError::JsonParseError(e))?;

                suite_result.add_evaluation(eval_result);

                // use current eval to determine where the summary should be written
                if summary_path.is_none() {
                    let mut result = PathBuf::new();
                    let mut iter = eval_path.components();
                    if let Some(first) = iter.next() {
                        result.push(first);
                        if let Some(second) = iter.next() {
                            result.push(second);
                        }
                    }
                    summary_path = Some(result);
                }
            }
            results.add_suite(suite_result);
        }

        if let Some(path) = summary_path {
            let mut run_summary = PathBuf::new();
            run_summary.push(path);
            run_summary.push(&self.config.run_summary_filename);

            let output_str = serde_json::to_string_pretty(&results)
                .map_err(|e| BenchError::JsonParseError(e))?;

            std::fs::write(run_summary, &output_str).map_err(|e| BenchError::IoError(e))?;
        }

        Ok(results)
    }

    fn collect_evals_for_run(&self) -> HashMap<String, Vec<BenchEval>> {
        // convert suites map {suite_name => [eval_selector_str] to map suite_name => [BenchEval]
        let mut result: HashMap<String, Vec<BenchEval>> = HashMap::new();
        for eval in self.config.evals.iter() {
            let selected_suites = EvaluationSuite::select(vec![eval.selector.clone()]);
            for (suite, evals) in selected_suites {
                let entry: &mut Vec<BenchEval> = result.entry(suite).or_default();
                entry.reserve(evals.len());
                for suite_eval in evals {
                    let mut updated_eval = eval.clone();
                    updated_eval.selector = suite_eval.to_string();
                    entry.push(updated_eval);
                }
            }
        }
        result
    }

    fn toolshim_envs(&self) -> Vec<(String, String)> {
        // read tool-shim preference from config, set respective env vars accordingly
        let mut shim_envs: Vec<(String, String)> = Vec::new();
        if let Some(model) = self.config.models.first() {
            if let Some(shim_opt) = &model.tool_shim {
                if shim_opt.use_tool_shim {
                    shim_envs.push(("GOOSE_TOOLSHIM".to_string(), "true".to_string()));
                    if let Some(shim_model) = &shim_opt.tool_shim_model {
                        shim_envs.push((
                            "GOOSE_TOOLSHIM_OLLAMA_MODEL".to_string(),
                            shim_model.clone(),
                        ));
                    }
                }
            }
        }
        shim_envs
    }

    fn load_env_file(path: &PathBuf) -> BenchResult<Vec<(String, String)>> {
        let iter = from_path_iter(path).map_err(|e| BenchError::DotenvyError(e))?;
        let env_vars = iter
            .map(|item| item.map_err(|e| BenchError::DotenvyError(e)))
            .collect::<Result<_, _>>()?;
        Ok(env_vars)
    }
}
