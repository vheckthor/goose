use crate::bench_config::BenchModel;
use crate::errors::{BenchError, BenchResult};
use crate::logging;
use crate::reporting::BenchmarkResults;
use polars::{io::csv::QuoteStyle, prelude::*};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

/// Handles DataFrame operations for benchmark results processing
pub struct DataFrameHandler;

impl DataFrameHandler {
    /// Sanitizes a filename by replacing characters that might cause issues
    pub fn sanitize_filename(name: &str) -> String {
        name.replace(':', "_").replace('"', "")
    }

    /// Generate CSV files from a benchmark directory
    pub fn generate_csv_from_benchmark_dir(benchmark_dir: &PathBuf) -> BenchResult<()> {
        logging::info(&format!(
            "Starting CSV generation from benchmark directory: {}",
            benchmark_dir.display()
        ));

        // Create eval-results directory
        let eval_results_dir = benchmark_dir.join("eval-results");
        fs::create_dir_all(&eval_results_dir).map_err(|e| BenchError::IoError(e))?;

        logging::info(&format!(
            "Created eval-results directory: {}",
            eval_results_dir.display()
        ));

        // Find all provider-model directories
        let entries: Vec<(String, PathBuf)> = fs::read_dir(benchmark_dir)
            .map_err(|e| BenchError::IoError(e))?
            .filter_map(|entry| {
                let entry = entry.ok()?;
                let path = entry.path();
                let dir_name = path.file_name()?.to_string_lossy().to_string();

                if path.is_dir() && dir_name.contains('-') && dir_name != "eval-results" {
                    Some((dir_name, path))
                } else {
                    None
                }
            })
            .collect();

        // Process each model directory
        for (dir_name, model_dir) in entries {
            let parts: Vec<&str> = dir_name.split('-').collect();
            if parts.len() < 2 {
                logging::warn(&format!(
                    "Skipping directory with invalid format: {}",
                    dir_name
                ));
                continue;
            }

            let provider = parts[0].to_string();
            let model_name = parts[1..].join("-");

            // Read all run results into a DataFrame
            match Self::process_model_directory(&model_dir, &provider, &model_name) {
                Ok(df) => {
                    if df.height() > 0 {
                        // Generate evaluation-specific CSVs
                        if let Err(e) = Self::generate_eval_csvs(&df, &eval_results_dir) {
                            logging::error(&format!(
                                "Error generating eval CSVs for {}: {}",
                                dir_name, e
                            ));
                        }

                        // Generate aggregate metrics
                        if let Err(e) = Self::generate_aggregate_metrics(&df, &eval_results_dir) {
                            logging::error(&format!(
                                "Error generating aggregate metrics for {}: {}",
                                dir_name, e
                            ));
                        }

                        logging::info(&format!("Generated CSV files for {}", dir_name));
                    } else {
                        logging::warn(&format!("No data found for {}", dir_name));
                    }
                }
                Err(e) => {
                    logging::error(&format!("Error processing directory {}: {}", dir_name, e))
                }
            }
        }

        logging::info(&format!(
            "CSV files generated in {}",
            eval_results_dir.display()
        ));
        Ok(())
    }

    /// Processes a model directory and returns a DataFrame with all run results
    pub fn process_model_directory(
        model_dir: &PathBuf,
        provider: &str,
        model_name: &str,
    ) -> BenchResult<DataFrame> {
        logging::info(&format!(
            "Processing model directory: {}",
            model_dir.display()
        ));
        // Find all run directories
        let run_dirs: Vec<(usize, PathBuf)> = fs::read_dir(model_dir)
            .map_err(|e| BenchError::IoError(e))?
            .filter_map(|entry| {
                let entry = entry.ok()?;
                let path = entry.path();
                if path.is_dir() {
                    let name = path.file_name()?.to_string_lossy();
                    if let Some(run_num) = name.strip_prefix("run-") {
                        if let Ok(idx) = run_num.parse::<usize>() {
                            return Some((idx, path));
                        }
                    }
                }
                None
            })
            .collect();

        if run_dirs.is_empty() {
            return Ok(DataFrame::default());
        }

        // Process each run directory and collect records
        let mut records = Vec::new();
        let mut metric_types = HashMap::new();

        for (run_idx, run_dir) in run_dirs {
            let summary_file = run_dir.join("run-results-summary.json");
            if !summary_file.exists() {
                continue;
            }

            let content = fs::read_to_string(&summary_file).map_err(|e| BenchError::IoError(e))?;

            let result: BenchmarkResults =
                serde_json::from_str(&content).map_err(|e| BenchError::JsonParseError(e))?;

            // Process results and track metric types
            for suite in &result.suites {
                for eval in &suite.evaluations {
                    let mut record = HashMap::new();

                    // Add metadata
                    record.insert("provider".to_string(), provider.to_string());
                    record.insert("model_name".to_string(), model_name.to_string());
                    record.insert("eval_suite".to_string(), suite.name.clone());
                    record.insert("eval_name".to_string(), eval.name.clone());
                    record.insert("run_num".to_string(), run_idx.to_string());

                    // Add metrics and track types
                    for (metric_name, value) in &eval.metrics {
                        if let Ok(_num) = value.to_string().parse::<f64>() {
                            metric_types
                                .entry(metric_name.clone())
                                .and_modify(|e: &mut DataType| {
                                    *e = DataType::Float64;
                                })
                                .or_insert(DataType::Float64);
                        } else {
                            metric_types
                                .entry(metric_name.clone())
                                .and_modify(|e: &mut DataType| {
                                    *e = DataType::Utf8;
                                })
                                .or_insert(DataType::Utf8);
                        }
                        record.insert(metric_name.clone(), value.to_string());
                    }

                    logging::info(&format!(
                        "Adding record for {}/{} with {} metrics",
                        suite.name,
                        eval.name,
                        eval.metrics.len()
                    ));
                    records.push(record);
                }
            }
        }

        if records.is_empty() {
            return Ok(DataFrame::default());
        }

        Self::create_dataframe_from_records(records)
    }

    /// Converts benchmark results to a DataFrame
    pub fn results_to_dataframe(
        results: &[BenchmarkResults],
        model: &BenchModel,
    ) -> BenchResult<DataFrame> {
        let mut records = Vec::new();

        for (run_idx, result) in results.iter().enumerate() {
            for suite in &result.suites {
                for eval in &suite.evaluations {
                    let mut record = HashMap::new();

                    // Add metadata
                    record.insert("provider".to_string(), model.provider.clone());
                    record.insert("model_name".to_string(), model.name.clone());
                    record.insert("eval_suite".to_string(), suite.name.clone());
                    record.insert("eval_name".to_string(), eval.name.clone());
                    record.insert("run_num".to_string(), run_idx.to_string());

                    // Add metrics
                    for (metric_name, value) in &eval.metrics {
                        record.insert(metric_name.clone(), value.to_string());
                    }

                    records.push(record);
                }
            }
        }

        if records.is_empty() {
            return Ok(DataFrame::default());
        }

        Self::create_dataframe_from_records(records)
    }

    /// Creates a DataFrame from a collection of records
    fn create_dataframe_from_records(
        records: Vec<HashMap<String, String>>,
    ) -> BenchResult<DataFrame> {
        // Get all column names
        let columns: HashSet<String> = records
            .iter()
            .flat_map(|record| record.keys().cloned())
            .collect();

        // Create series for each column
        let mut series_vec = Vec::new();

        // Add metadata columns first with known types
        let provider_values: Vec<&str> = records
            .iter()
            .map(|record| record.get("provider").map(String::as_str).unwrap_or(""))
            .collect();
        series_vec.push(Series::new("provider", provider_values));

        let model_name_values: Vec<&str> = records
            .iter()
            .map(|record| record.get("model_name").map(String::as_str).unwrap_or(""))
            .collect();
        series_vec.push(Series::new("model_name", model_name_values));

        let eval_suite_values: Vec<&str> = records
            .iter()
            .map(|record| record.get("eval_suite").map(String::as_str).unwrap_or(""))
            .collect();
        series_vec.push(Series::new("eval_suite", eval_suite_values));

        let eval_name_values: Vec<&str> = records
            .iter()
            .map(|record| record.get("eval_name").map(String::as_str).unwrap_or(""))
            .collect();
        series_vec.push(Series::new("eval_name", eval_name_values));

        let run_num_values: Vec<i64> = records
            .iter()
            .map(|record| {
                record
                    .get("run_num")
                    .and_then(|v| v.parse::<i64>().ok())
                    .unwrap_or(0)
            })
            .collect();
        series_vec.push(Series::new("run_num", run_num_values));

        // Add metric columns with inferred types
        for column in columns.iter().filter(|c| {
            ![
                "provider",
                "model_name",
                "eval_suite",
                "eval_name",
                "run_num",
            ]
            .contains(&c.as_str())
        }) {
            let values: Vec<&str> = records
                .iter()
                .map(|record| record.get(column).map(String::as_str).unwrap_or(""))
                .collect();

            // Try to convert to appropriate type
            if column == "total_tool_calls"
                || column == "total_tokens"
                || column.starts_with("tool_calls_")
            {
                // Handle integer columns
                if let Ok(int_values) = values
                    .iter()
                    .map(|&v| v.parse::<i64>())
                    .collect::<Result<Vec<i64>, _>>()
                {
                    series_vec.push(Series::new(column, int_values));
                } else {
                    series_vec.push(Series::new(column, values));
                }
            } else if column.starts_with("Complete")
                || column.starts_with("Git")
                || column.ends_with("added")
                || column.ends_with("executed")
            {
                // Handle boolean columns as strings to maintain consistency
                series_vec.push(Series::new(column, values));
            } else if let Ok(float_values) = values
                .iter()
                .map(|&v| v.parse::<f64>())
                .collect::<Result<Vec<f64>, _>>()
            {
                series_vec.push(Series::new(column, float_values));
            } else {
                series_vec.push(Series::new(column, values));
            }
        }

        DataFrame::new(series_vec)
            .map_err(|e| BenchError::DataFrameError(format!("Failed to create DataFrame: {}", e)))
    }

    /// Generates CSV files for each unique evaluation
    pub fn generate_eval_csvs(df: &DataFrame, output_dir: &Path) -> BenchResult<()> {
        logging::info(&format!(
            "Starting generate_eval_csvs with DataFrame of height: {}",
            df.height()
        ));

        // Important columns that should come first in the CSV
        let important_columns = [
            "provider",
            "model_name",
            "eval_suite",
            "eval_name",
            "run_num",
            "total_tool_calls",
            "prompt_execution_time_seconds",
            "total_tokens",
        ];

        // Get unique combinations of eval_suite and eval_name
        let unique_evals = df
            .clone()
            .lazy()
            .select([col("eval_suite"), col("eval_name")])
            .unique(
                Some(vec!["eval_suite".to_string(), "eval_name".to_string()]),
                UniqueKeepStrategy::First,
            )
            .collect()
            .map_err(|e| {
                BenchError::DataFrameError(format!("Failed to get unique evaluations: {}", e))
            })?;

        logging::info(&format!(
            "unique_evals top rows are: {:?}",
            unique_evals.head(Some(5))
        ));

        for row_idx in 0..unique_evals.height() {
            let suite_name = unique_evals
                .column("eval_suite")
                .map_err(|e| {
                    BenchError::DataFrameError(format!("Failed to get eval_suite column: {}", e))
                })?
                .get(row_idx)
                .unwrap()
                .get_str()
                .unwrap()
                .to_string();

            let eval_name = unique_evals
                .column("eval_name")
                .map_err(|e| {
                    BenchError::DataFrameError(format!("Failed to get eval_name column: {}", e))
                })?
                .get(row_idx)
                .unwrap()
                .get_str()
                .unwrap()
                .to_string();

            logging::info(&format!(
                "Processing evaluation: {}/{}",
                suite_name, eval_name
            ));

            // Filter DataFrame for current eval
            let eval_df = df
                .clone()
                .lazy()
                .filter(
                    col("eval_suite")
                        .eq(lit(suite_name.as_str()))
                        .and(col("eval_name").eq(lit(eval_name.as_str()))),
                )
                .collect()
                .map_err(|e| {
                    BenchError::DataFrameError(format!("Failed to filter DataFrame: {}", e))
                })?;

            if eval_df.height() == 0 {
                logging::warn(&format!(
                    "No rows found for evaluation: {}/{}",
                    suite_name, eval_name
                ));
                continue;
            }

            // Create the CSV file path
            let sanitized_suite = Self::sanitize_filename(&suite_name);
            let sanitized_eval = Self::sanitize_filename(&eval_name);
            let file_path = output_dir.join(format!("{}_{}.csv", sanitized_suite, sanitized_eval));

            logging::info(&format!("Writing to file: {}", file_path.display()));

            // Get all column names and sort them with important columns first
            let mut columns: Vec<String> = eval_df
                .get_column_names()
                .iter()
                .map(|&s| s.to_string())
                .collect();

            columns.sort_by(|a, b| {
                let a_idx = important_columns.iter().position(|c| c == a);
                let b_idx = important_columns.iter().position(|c| c == b);

                match (a_idx, b_idx) {
                    (Some(a_i), Some(b_i)) => a_i.cmp(&b_i),
                    (Some(_), None) => std::cmp::Ordering::Less,
                    (None, Some(_)) => std::cmp::Ordering::Greater,
                    (None, None) => a.cmp(b),
                }
            });

            // Create a new DataFrame with reordered columns
            let mut ordered_df = eval_df.select(&columns).map_err(|e| {
                BenchError::DataFrameError(format!("Failed to reorder columns: {}", e))
            })?;

            // Check if file exists and merge with existing data if it does
            if file_path.exists() {
                logging::info("Found existing file, merging data...");
                // Read existing CSV and cast boolean columns to strings
                let mut existing_df = CsvReader::from_path(&file_path)
                    .map_err(|e| {
                        BenchError::DataFrameError(format!("Failed to open CSV file: {}", e))
                    })?
                    .has_header(true)
                    .finish()
                    .map_err(|e| {
                        BenchError::DataFrameError(format!("Failed to read existing CSV: {}", e))
                    })?;

                // Convert any boolean columns to strings
                let column_names: Vec<String> = existing_df
                    .get_column_names()
                    .iter()
                    .map(|&s| s.to_string())
                    .collect();

                for col_name in column_names {
                    if let Ok(col) = existing_df.column(&col_name) {
                        if matches!(col.dtype(), DataType::Boolean) {
                            let str_values: Vec<String> = col
                                .bool()
                                .map_err(|e| {
                                    BenchError::DataFrameError(format!(
                                        "Failed to convert boolean column: {}",
                                        e
                                    ))
                                })?
                                .into_iter()
                                .map(|opt_val| opt_val.map(|v| v.to_string()).unwrap_or_default())
                                .collect();

                            existing_df
                                .replace(&col_name, Series::new(&col_name, str_values))
                                .map_err(|e| {
                                    BenchError::DataFrameError(format!(
                                        "Failed to replace column: {}",
                                        e
                                    ))
                                })?;
                        }
                    }
                }

                // Combine existing and new data, dropping duplicates
                let concat_df = concat(
                    &[existing_df.lazy(), ordered_df.lazy()],
                    UnionArgs {
                        parallel: true,
                        ..Default::default()
                    },
                )
                .map_err(|e| {
                    BenchError::DataFrameError(format!("Failed to concatenate DataFrames: {}", e))
                })?;

                ordered_df = concat_df
                    .unique(
                        Some(vec![
                            "provider".to_string(),
                            "model_name".to_string(),
                            "eval_suite".to_string(),
                            "eval_name".to_string(),
                            "run_num".to_string(),
                        ]),
                        UniqueKeepStrategy::First,
                    )
                    .collect()
                    .map_err(|e| {
                        BenchError::DataFrameError(format!(
                            "Failed to deduplicate merged data: {}",
                            e
                        ))
                    })?;
            }

            // Write to CSV atomically using a temporary file
            let temp_path: PathBuf = file_path.with_extension("csv.tmp");
            let file = std::fs::File::create(&temp_path).map_err(|e| BenchError::IoError(e))?;

            CsvWriter::new(&file)
                .include_header(true)
                .with_separator(b',')
                .with_quote_style(QuoteStyle::NonNumeric)
                .finish(&mut ordered_df)
                .map_err(|e| BenchError::DataFrameError(format!("Failed to write CSV: {}", e)))?;

            // Atomically rename temp file to final file
            fs::rename(temp_path, file_path).map_err(|e| BenchError::IoError(e))?;

            logging::info(&format!(
                "Successfully wrote file with {} rows",
                ordered_df.height()
            ));
        }

        Ok(())
    }

    /// Generates aggregate metrics from a DataFrame
    pub fn generate_aggregate_metrics(df: &DataFrame, output_dir: &Path) -> BenchResult<()> {
        // Identify numeric columns (excluding run_num)
        let numeric_cols: Vec<String> = df
            .get_columns()
            .iter()
            .filter(|col| {
                matches!(
                    col.dtype(),
                    DataType::Int64 | DataType::Float64 | DataType::UInt64
                ) && col.name() != "run_num"
            })
            .map(|col| col.name().to_string())
            .collect();

        logging::info(&format!("Numeric columns found: {:?}", numeric_cols));

        // Debug: Print all column types
        for col in df.get_columns() {
            logging::debug(&format!(
                "Column '{}' has type: {:?}",
                col.name(),
                col.dtype()
            ));
        }

        // Create aggregation expressions
        let mut agg_exprs: Vec<Expr> = vec![];

        for numeric_col in &numeric_cols {
            agg_exprs.push(
                col(numeric_col)
                    .mean()
                    .alias(&format!("{}_mean", numeric_col)),
            );
        }

        // Perform aggregation
        let mut agg_df = df
            .clone()
            .lazy()
            .group_by([
                col("provider"),
                col("model_name"),
                col("eval_suite"),
                col("eval_name"),
            ])
            .agg(agg_exprs)
            .collect()
            .map_err(|e| {
                BenchError::DataFrameError(format!("Failed to aggregate metrics: {}", e))
            })?;

        // Write to CSV
        let file = std::fs::File::create(output_dir.join("aggregate_metrics.csv"))
            .map_err(|e| BenchError::IoError(e))?;

        CsvWriter::new(&file)
            .include_header(true)
            .with_separator(b',')
            .with_quote_style(QuoteStyle::NonNumeric)
            .finish(&mut agg_df)
            .map_err(|e| {
                BenchError::DataFrameError(format!("Failed to write aggregate metrics: {}", e))
            })?;

        Ok(())
    }
}
