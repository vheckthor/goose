use crate::bench_config::{BenchEval, BenchModel, BenchRunConfig};
use crate::dataframe_handler::DataFrameHandler;
use crate::errors::{BenchError, BenchResult};
use crate::eval_suites::EvaluationSuite;
use crate::logging;
use crate::reporting::{BenchmarkResults, SuiteResult};
use crate::runners::eval_runner::EvalRunner;
use crate::utilities::{await_process_exits, parallel_bench_cmd};
use std::collections::HashMap;
use std::fs::{self, read_to_string};
use std::io::{self, BufRead};
use std::path::PathBuf;
use std::process::Child;
use std::thread;

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

    pub fn generate_csv_from_benchmark_dir(benchmark_dir: &PathBuf) -> BenchResult<()> {
        DataFrameHandler::generate_csv_from_benchmark_dir(benchmark_dir)
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
                    logging::error(&format!("Failed to collect results for run {}: {}", i, e))
                }
            }
        }

        // Create DataFrame from results and generate reports
        if let Ok(df) = DataFrameHandler::results_to_dataframe(&all_runs_results, model) {
            // Determine output directory
            if let Some(eval_results_dir) =
                self.determine_output_directory(&all_runs_results, model)
            {
                fs::create_dir_all(&eval_results_dir).map_err(|e| BenchError::IoError(e))?;

                DataFrameHandler::generate_eval_csvs(&df, &eval_results_dir).map_err(|e| {
                    BenchError::ResultsProcessingError(format!("CSV generation failed: {}", e))
                })?;

                DataFrameHandler::generate_aggregate_metrics(&df, &eval_results_dir).map_err(
                    |e| {
                        BenchError::ResultsProcessingError(format!(
                            "Metrics generation failed: {}",
                            e
                        ))
                    },
                )?;
            }
        }

        Ok(())
    }

    fn determine_output_directory(
        &self,
        results: &[BenchmarkResults],
        model: &BenchModel,
    ) -> Option<PathBuf> {
        if let Some(first_run) = results.first() {
            if let Some(first_suite) = first_run.suites.first() {
                if let Some(first_eval) = first_suite.evaluations.first() {
                    let eval_path = EvalRunner::path_for_eval(
                        model,
                        &BenchEval {
                            selector: format!("{}:{}", first_suite.name, first_eval.name),
                            post_process_cmd: None,
                            parallel_safe: true,
                        },
                        "0".to_string(),
                    );
                    // eval_path is like: databricks-goose/run-0/core/developer_list_files
                    // We need to find the parent directory that contains the run-x directories
                    let mut current_path = eval_path.as_path();
                    let mut model_dir = None;

                    while let Some(parent) = current_path.parent() {
                        if parent
                            .file_name()
                            .and_then(|name| name.to_str())
                            .map_or(false, |name| name.starts_with("run-"))
                        {
                            // Found a run-x directory, so its parent is the model directory
                            model_dir = parent.parent().map(|p| p.to_path_buf());
                            break;
                        }
                        current_path = parent;
                    }

                    model_dir.map(|dir| dir.join("eval-results"))
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        }
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
            let env_vars = self.load_env_file(env_file).map_err(|e| {
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

    fn load_env_file(&self, path: &PathBuf) -> BenchResult<Vec<(String, String)>> {
        let file = std::fs::File::open(path).map_err(|e| BenchError::IoError(e))?;

        let reader = io::BufReader::new(file);
        let mut env_vars = Vec::new();

        for line in reader.lines() {
            let line = line.map_err(|e| BenchError::IoError(e))?;

            // Skip empty lines and comments
            if line.trim().is_empty() || line.trim_start().starts_with('#') {
                continue;
            }

            // Split on first '=' only
            if let Some((key, value)) = line.split_once('=') {
                let key = key.trim().to_string();
                // Remove quotes if present
                let value = value
                    .trim()
                    .trim_matches('"')
                    .trim_matches('\'')
                    .to_string();
                env_vars.push((key, value));
            }
        }

        Ok(env_vars)
    }
}
