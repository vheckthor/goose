use crate::bench_config::BenchModel;
use crate::dataframe_handler::DataFrameHandler;
use crate::errors::BenchResult;
use crate::reporting::types::BenchmarkResults;
use std::fs;
use std::path::{Path, PathBuf};

/// Trait for report generators
pub trait ReportGenerator {
    fn generate(&self, results: &[BenchmarkResults], output_dir: &Path) -> BenchResult<()>;
}

/// Generates CSV reports from benchmark results
pub struct CsvReportGenerator {
    model: BenchModel,
}

impl CsvReportGenerator {
    pub fn new(model: BenchModel) -> Self {
        Self { model }
    }
}

impl ReportGenerator for CsvReportGenerator {
    fn generate(&self, results: &[BenchmarkResults], output_dir: &Path) -> BenchResult<()> {
        // Ensure the output directory exists
        fs::create_dir_all(output_dir)?;

        // Convert results to DataFrame
        let df = DataFrameHandler::results_to_dataframe(results, &self.model)?;

        if df.height() == 0 {
            return Ok(());
        }

        // Generate per-evaluation CSV files
        DataFrameHandler::generate_eval_csvs(&df, output_dir)?;

        Ok(())
    }
}

/// Generates aggregate metrics reports
pub struct MetricsReportGenerator;

impl MetricsReportGenerator {
    pub fn new() -> Self {
        Self
    }
}

impl ReportGenerator for MetricsReportGenerator {
    fn generate(&self, results: &[BenchmarkResults], output_dir: &Path) -> BenchResult<()> {
        // Ensure the output directory exists
        fs::create_dir_all(output_dir)?;

        // Convert benchmark results to DataFrame using model from first result
        if let Some(first_result) = results.first() {
            let model = BenchModel {
                provider: first_result.provider.clone(),
                name: "unknown".to_string(), // We don't have the model name in BenchmarkResults
                parallel_safe: true,
                tool_shim: None,
            };

            let df = DataFrameHandler::results_to_dataframe(results, &model)?;

            if df.height() == 0 {
                return Ok(());
            }

            // Generate aggregate metrics
            DataFrameHandler::generate_aggregate_metrics(&df, output_dir)?;
        }

        Ok(())
    }
}

/// Utility function to generate CSV files from a benchmark directory
pub fn generate_csv_from_benchmark_dir(benchmark_dir: &PathBuf) -> BenchResult<()> {
    DataFrameHandler::generate_csv_from_benchmark_dir(benchmark_dir)
}
