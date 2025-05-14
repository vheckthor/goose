use crate::bench_config::{BenchEval, BenchModel};
use crate::config_manager::ConfigManager;
use crate::errors::{BenchError, BenchResult};
use crate::eval_suites::EvaluationSuite;
use crate::logging;
use crate::reporting::{
    BenchmarkResults, CsvReportGenerator, MetricsReportGenerator, ReportGenerator,
};
use crate::runners::eval_runner::EvalRunner;
use crate::runners::result_collector::ResultCollector;
use crate::utilities::{await_process_exits, parallel_bench_cmd};
use std::collections::HashMap;
use std::fs;
use std::process::Child;
use std::thread;

/// Main benchmark runner that coordinates the benchmark process
pub struct BenchmarkRunner {
    config_manager: ConfigManager,
    result_collector: ResultCollector,
}

impl BenchmarkRunner {
    /// Create a new BenchmarkRunner from a configuration string
    pub fn from_string(config: String) -> BenchResult<Self> {
        let config_manager = ConfigManager::from_string(config)
            .map_err(|e| BenchError::ConfigError(format!("Failed to parse config: {}", e)))?;

        let result_collector = ResultCollector::new();

        Ok(Self {
            config_manager,
            result_collector,
        })
    }

    /// Run the benchmark
    pub fn run(&mut self) -> BenchResult<()> {
        // Get the model to benchmark
        let model = self
            .config_manager
            .config()
            .models
            .first()
            .ok_or_else(|| BenchError::ConfigError("No model specified in config".to_string()))?
            .clone();

        // Collect and organize evaluations to run
        let suites = self.collect_evals_for_run();
        logging::info(&format!(
            "Running benchmarks for model {}/{} with {} evaluation suites",
            model.provider,
            model.name,
            suites.len()
        ));

        // Run the specified number of repeats
        let repeat_count = self.config_manager.config().repeat.unwrap_or(1);
        logging::info(&format!("Will run each evaluation {} times", repeat_count));

        let mut handles = vec![];

        // Start the benchmark runs
        for i in 0..repeat_count {
            let run_id = i.to_string();
            logging::info(&format!("Starting benchmark run {}", run_id));

            // Clone required data for the thread
            let runner_copy = self.clone();
            let model_clone = model.clone();
            let suites_clone = suites.clone();

            // Run in a separate thread
            let handle = thread::spawn(move || {
                runner_copy.run_benchmark(&model_clone, suites_clone, run_id)
            });
            handles.push(handle);
        }

        // Wait for all threads to complete
        await_process_exits(&mut Vec::new(), handles);

        // Collect results from all runs
        let mut all_runs_results: Vec<BenchmarkResults> = Vec::new();
        for i in 0..repeat_count {
            let run_id = i.to_string();
            match self.result_collector.collect_run_results(
                &model,
                &suites,
                run_id,
                self.config_manager.config().eval_result_filename.clone(),
                self.config_manager.config().run_summary_filename.clone(),
            ) {
                Ok(results) => all_runs_results.push(results),
                Err(e) => {
                    logging::error(&format!("Failed to collect results for run {}: {}", i, e))
                }
            }
        }

        // Generate reports from the results
        self.generate_reports(&all_runs_results, &model)?;

        Ok(())
    }

    /// Generate reports from benchmark results
    fn generate_reports(
        &self,
        results: &[BenchmarkResults],
        model: &BenchModel,
    ) -> BenchResult<()> {
        if results.is_empty() {
            logging::warn("No benchmark results to generate reports from");
            return Ok(());
        }

        // Determine output directory
        let eval_results_dir = if let Some(first_run) = results.first() {
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
                    // Navigate up to find the model directory
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
        };

        if let Some(dir) = eval_results_dir {
            fs::create_dir_all(&dir).map_err(|e| BenchError::IoError(e))?;

            // Create and use report generators
            let csv_generator = CsvReportGenerator::new(model.clone());
            let metrics_generator = MetricsReportGenerator::new();

            csv_generator.generate(results, &dir).map_err(|e| {
                BenchError::ResultsProcessingError(format!("CSV generation failed: {}", e))
            })?;

            metrics_generator.generate(results, &dir).map_err(|e| {
                BenchError::ResultsProcessingError(format!("Metrics generation failed: {}", e))
            })?;

            logging::info(&format!("Generated reports in {}", dir.display()));
        } else {
            logging::warn("Could not determine output directory for reports");
        }

        Ok(())
    }

    /// Clone self for threading
    fn clone(&self) -> Self {
        // We need to manually implement clone since we don't derive it
        let config_str = self
            .config_manager
            .config()
            .to_string()
            .expect("Failed to serialize config");

        let config_manager =
            ConfigManager::from_string(config_str).expect("Failed to parse config");

        Self {
            config_manager,
            result_collector: ResultCollector::new(),
        }
    }

    /// Run the benchmark for a specific model, suite, and run ID
    fn run_benchmark(
        self,
        model: &BenchModel,
        suites: HashMap<String, Vec<BenchEval>>,
        run_id: String,
    ) -> BenchResult<()> {
        let mut results_handles = HashMap::<String, Vec<Child>>::new();

        // Prepare environment variables
        let mut envs = self.config_manager.get_toolshim_environment();
        let all_envs = self.config_manager.get_environment_variables();
        envs.extend(all_envs.into_iter());
        envs.push(("GOOSE_MODEL".to_string(), model.clone().name));
        envs.push(("GOOSE_PROVIDER".to_string(), model.clone().provider));

        // Only run in parallel if the model is parallel_safe
        let run_parallel = model.parallel_safe;
        logging::info(&format!("Running with parallel_safe={}", run_parallel));

        for (suite, evals) in suites.iter() {
            results_handles.insert((*suite).clone(), Vec::new());
            logging::info(&format!(
                "Processing suite {} with {} evaluations",
                suite,
                evals.len()
            ));

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
                logging::info(&format!(
                    "Running {} evaluations in parallel",
                    parallel_evals.len()
                ));
                for eval_selector in &parallel_evals {
                    let cfg = self
                        .config_manager
                        .create_eval_config(eval_selector, run_id.clone())?;
                    let handle = parallel_bench_cmd("exec-eval".to_string(), cfg, envs.clone());
                    results_handles.get_mut(suite).unwrap().push(handle);
                }
            }

            // Run non-parallel-safe evaluations sequentially
            if !sequential_evals.is_empty() {
                logging::info(&format!(
                    "Running {} evaluations sequentially",
                    sequential_evals.len()
                ));
                for eval_selector in &sequential_evals {
                    let cfg = self
                        .config_manager
                        .create_eval_config(eval_selector, run_id.clone())?;
                    let handle = parallel_bench_cmd("exec-eval".to_string(), cfg, envs.clone());

                    // Wait for this process to complete before starting the next one
                    let mut child_procs = vec![handle];
                    await_process_exits(&mut child_procs, Vec::new());
                }
            }
        }

        // Wait for any remaining parallel processes to complete
        for (suite_name, child_procs) in results_handles.iter_mut() {
            logging::info(&format!(
                "Waiting for {} remaining processes in suite {}",
                child_procs.len(),
                suite_name
            ));
            await_process_exits(child_procs, Vec::new());
        }

        Ok(())
    }

    /// Collect and organize evaluations to run
    fn collect_evals_for_run(&self) -> HashMap<String, Vec<BenchEval>> {
        // Convert suites map {suite_name => [eval_selector_str] to map suite_name => [BenchEval]
        let mut result: HashMap<String, Vec<BenchEval>> = HashMap::new();
        for eval in self.config_manager.config().evals.iter() {
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
}
