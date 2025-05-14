use crate::bench_config::{BenchEval, BenchModel};
use crate::errors::BenchResult;
use crate::logging;
use crate::reporting::{BenchmarkResults, SuiteResult};
use crate::runners::eval_runner::EvalRunner;
use std::collections::HashMap;
use std::fs::read_to_string;
use std::path::PathBuf;

/// Collects and manages benchmark results
pub struct ResultCollector;

impl ResultCollector {
    /// Create a new ResultCollector
    pub fn new() -> Self {
        Self
    }

    /// Collect results from a completed benchmark run
    pub fn collect_run_results(
        &self,
        model: &BenchModel,
        suites: &HashMap<String, Vec<BenchEval>>,
        run_id: String,
        eval_result_filename: String,
        run_summary_filename: String,
    ) -> BenchResult<BenchmarkResults> {
        logging::info(&format!("Collecting results for run {}", run_id));
        let mut results = BenchmarkResults::new(model.provider.clone());
        let mut summary_path: Option<PathBuf> = None;

        for (suite, evals) in suites.iter() {
            logging::debug(&format!(
                "Processing suite {} with {} evaluations",
                suite,
                evals.len()
            ));
            let mut suite_result = SuiteResult::new(suite.clone());

            for eval_selector in evals {
                let mut eval_path = EvalRunner::path_for_eval(model, eval_selector, run_id.clone());
                eval_path.push(eval_result_filename.clone());

                if !eval_path.exists() {
                    logging::warn(&format!(
                        "Evaluation result file not found at: {}",
                        eval_path.display()
                    ));
                    continue;
                }

                match read_to_string(&eval_path) {
                    Ok(content) => match serde_json::from_str(&content) {
                        Ok(eval_result) => {
                            suite_result.add_evaluation(eval_result);
                            logging::debug(&format!(
                                "Added evaluation result from {}",
                                eval_path.display()
                            ));
                        }
                        Err(e) => {
                            logging::error(&format!(
                                "Failed to parse evaluation result from {}: {}",
                                eval_path.display(),
                                e
                            ));
                        }
                    },
                    Err(e) => {
                        logging::error(&format!(
                            "Failed to read evaluation result from {}: {}",
                            eval_path.display(),
                            e
                        ));
                    }
                }

                // Use current eval to determine where the summary should be written
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

        // Write the summary file
        if let Some(path) = summary_path {
            let mut run_summary = path.clone();
            run_summary.push(&run_summary_filename);

            match serde_json::to_string_pretty(&results) {
                Ok(output_str) => match std::fs::write(&run_summary, &output_str) {
                    Ok(_) => {
                        logging::info(&format!("Wrote run summary to {}", run_summary.display()))
                    }
                    Err(e) => logging::error(&format!(
                        "Failed to write run summary to {}: {}",
                        run_summary.display(),
                        e
                    )),
                },
                Err(e) => logging::error(&format!("Failed to serialize run results: {}", e)),
            }
        } else {
            logging::warn("No summary path determined, skipping summary file writing");
        }

        Ok(results)
    }
}
