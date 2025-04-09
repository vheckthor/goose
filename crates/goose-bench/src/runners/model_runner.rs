use crate::bench_config::{BenchEval, BenchModel, BenchRunConfig};
use crate::eval_suites::EvaluationSuite;
use crate::reporting::{BenchmarkResults, SuiteResult};
use crate::runners::eval_runner::EvalRunner;
use crate::utilities::{await_process_exits, parallel_bench_cmd, union_hashmaps};
use anyhow::Context;
use polars::prelude::*;
use std::collections::{HashMap, HashSet};
use std::fs::{self, read_to_string};
use std::io::{self, BufRead, Write};
use std::path::PathBuf;
use std::process::Child;
use std::thread;

#[derive(Clone)]
pub struct ModelRunner {
    config: BenchRunConfig,
}

impl ModelRunner {
    pub fn from(config: String) -> anyhow::Result<ModelRunner> {
        let config = BenchRunConfig::from_string(config)?;
        Ok(ModelRunner { config })
    }

    fn sanitize_filename(name: &str) -> String {
        name.replace(':', "_").replace('"', "")
    }

    pub fn generate_csv_from_benchmark_dir(benchmark_dir: &PathBuf) -> anyhow::Result<()> {
        println!(
            "Starting CSV generation from benchmark directory: {}",
            benchmark_dir.display()
        );

        // Create eval-results directory
        let eval_results_dir = benchmark_dir.join("eval-results");
        fs::create_dir_all(&eval_results_dir)?;
        println!(
            "Created eval-results directory: {}",
            eval_results_dir.display()
        );

        // Find all provider-model directories
        let entries: Vec<(String, PathBuf)> = fs::read_dir(benchmark_dir)?
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
                println!("Skipping directory with invalid format: {}", dir_name);
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
                            println!("Error generating eval CSVs for {}: {}", dir_name, e);
                        }

                        // Generate aggregate metrics
                        if let Err(e) = Self::generate_aggregate_metrics(&df, &eval_results_dir) {
                            println!("Error generating aggregate metrics for {}: {}", dir_name, e);
                        }

                        println!("Generated CSV files for {}", dir_name);
                    } else {
                        println!("No data found for {}", dir_name);
                    }
                }
                Err(e) => println!("Error processing directory {}: {}", dir_name, e),
            }
        }

        println!("CSV files generated in {}", eval_results_dir.display());
        Ok(())
    }

    fn process_model_directory(
        model_dir: &PathBuf,
        provider: &str,
        model_name: &str,
    ) -> anyhow::Result<DataFrame> {
        println!("Processing model directory: {}", model_dir.display());
        // Find all run directories
        let run_dirs: Vec<(usize, PathBuf)> = fs::read_dir(model_dir)?
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

            let content = fs::read_to_string(&summary_file)?;
            let result: BenchmarkResults = serde_json::from_str(&content)?;

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

                    println!(
                        "Adding record for {}/{} with {} metrics",
                        suite.name,
                        eval.name,
                        eval.metrics.len()
                    );
                    records.push(record);
                }
            }
        }

        if records.is_empty() {
            return Ok(DataFrame::default());
        }

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

        let run_num_values: Vec<u64> = records
            .iter()
            .map(|record| {
                record
                    .get("run_num")
                    .and_then(|v| v.parse::<u64>().ok())
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

            // Try to convert to numeric if possible
            if let Ok(float_values) = values
                .iter()
                .map(|&v| v.parse::<f64>())
                .collect::<Result<Vec<f64>, _>>()
            {
                series_vec.push(Series::new(column, float_values));
            } else {
                series_vec.push(Series::new(column, values));
            }
        }

        DataFrame::new(series_vec).context("Failed to create DataFrame")
    }

    fn generate_eval_csvs(df: &DataFrame, output_dir: &PathBuf) -> anyhow::Result<()> {
        println!(
            "Starting generate_eval_csvs with DataFrame of height: {}",
            df.height()
        );

        // Get the benchmark directory from the output_dir
        let benchmark_dir = output_dir.parent().unwrap_or(output_dir);

        // Find all provider-model directories
        let entries: Vec<(String, PathBuf)> = fs::read_dir(benchmark_dir)?
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
                println!("Skipping directory with invalid format: {}", dir_name);
                continue;
            }

            let provider = parts[0].to_string();
            let model_name = parts[1..].join("-");

            // Find all run directories
            let run_dirs: Vec<(usize, PathBuf)> = fs::read_dir(&model_dir)?
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

            // Create a map to store data for each evaluation
            let mut eval_data: HashMap<String, Vec<HashMap<String, String>>> = HashMap::new();

            // Process each run directory
            for (run_idx, run_dir) in &run_dirs {
                let summary_file = run_dir.join("run-results-summary.json");
                if !summary_file.exists() {
                    println!("Summary file not found: {}", summary_file.display());
                    continue;
                }

                // Read and parse the summary file
                let content = fs::read_to_string(&summary_file)?;
                let result: BenchmarkResults = serde_json::from_str(&content)?;

                // Process each suite and evaluation
                for suite in &result.suites {
                    for eval in &suite.evaluations {
                        // Create a key for this evaluation
                        let eval_key = format!("{}_{}", suite.name, eval.name);

                        // Create a record for this run
                        let mut record = HashMap::new();
                        record.insert("provider".to_string(), provider.clone());
                        record.insert("model_name".to_string(), model_name.clone());
                        record.insert("eval_suite".to_string(), suite.name.clone());
                        record.insert("eval_name".to_string(), eval.name.clone());
                        record.insert("run_num".to_string(), run_idx.to_string());

                        // Add metrics
                        for (metric_name, value) in &eval.metrics {
                            record.insert(metric_name.clone(), value.to_string());
                        }

                        // Add to the eval_data map
                        eval_data
                            .entry(eval_key)
                            .or_insert_with(Vec::new)
                            .push(record);
                    }
                }
            }

            // Write CSV files for each evaluation
            for (eval_key, records) in eval_data {
                if records.is_empty() {
                    continue;
                }

                // Get the suite and eval names from the first record
                let suite_name = records[0].get("eval_suite").unwrap().clone();
                let eval_name = records[0].get("eval_name").unwrap().clone();

                println!("Processing evaluation: {}/{}", suite_name, eval_name);

                // Create the CSV file
                let sanitized_suite = Self::sanitize_filename(&suite_name);
                let sanitized_eval = Self::sanitize_filename(&eval_name);
                let file_path =
                    output_dir.join(format!("{}_{}.csv", sanitized_suite, sanitized_eval));

                println!("Writing to file: {}", file_path.display());

                // Write directly to the file using standard Rust file I/O
                let file = std::fs::File::create(&file_path)?;
                let mut writer = std::io::BufWriter::new(file);

                // Get all column names
                let mut columns = HashSet::new();
                for record in &records {
                    for key in record.keys() {
                        columns.insert(key.clone());
                    }
                }

                // Sort columns to ensure consistent order
                let mut column_list: Vec<String> = columns.into_iter().collect();
                column_list.sort();

                // Ensure important columns come first
                let important_columns = vec![
                    "provider",
                    "model_name",
                    "eval_suite",
                    "eval_name",
                    "run_num",
                    "total_tool_calls",
                    "prompt_execution_time_seconds",
                    "total_tokens",
                ];

                // Reorder columns to put important ones first
                column_list.sort_by(|a, b| {
                    let a_idx = important_columns.iter().position(|c| c == a);
                    let b_idx = important_columns.iter().position(|c| c == b);

                    match (a_idx, b_idx) {
                        (Some(a_i), Some(b_i)) => a_i.cmp(&b_i),
                        (Some(_), None) => std::cmp::Ordering::Less,
                        (None, Some(_)) => std::cmp::Ordering::Greater,
                        (None, None) => a.cmp(b),
                    }
                });

                // Write CSV header
                writeln!(writer, "{}", column_list.join(","))?;

                // Write data rows
                for record in records {
                    let row: Vec<String> = column_list
                        .iter()
                        .map(|col| record.get(col).cloned().unwrap_or_default())
                        .collect();

                    writeln!(writer, "{}", row.join(","))?;
                }

                // Flush the writer to ensure all data is written
                writer.flush()?;

                println!("Successfully wrote file: {}", file_path.display());
            }
        }

        Ok(())
    }

    fn generate_aggregate_metrics(df: &DataFrame, output_dir: &PathBuf) -> anyhow::Result<()> {
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
            .collect()?;

        // Write to CSV
        let mut file = std::fs::File::create(&output_dir.join("aggregate_metrics.csv"))?;
        CsvWriter::new(&mut file)
            .include_header(true)
            .with_separator(b',')
            .finish(&mut agg_df)?;

        Ok(())
    }

    pub fn run(&self) -> anyhow::Result<()> {
        let model = self.config.models.first().unwrap();
        let suites = self.collect_evals_for_run();

        let mut handles = vec![];

        for i in 0..self.config.repeat.unwrap_or(1) {
            let mut self_copy = self.clone();
            let model_clone = model.clone();
            let suites_clone = suites.clone();
            let handle = thread::spawn(move || {
                self_copy.run_benchmark(&model_clone, suites_clone, i.to_string())
            });
            handles.push(handle);
        }
        await_process_exits(&mut Vec::new(), handles);

        let mut all_runs_results: Vec<BenchmarkResults> = Vec::new();
        for i in 0..self.config.repeat.unwrap_or(1) {
            let run_results =
                self.collect_run_results(model.clone(), suites.clone(), i.to_string())?;
            all_runs_results.push(run_results);
        }

        // Create DataFrame from results and generate reports
        let df = Self::results_to_dataframe(&all_runs_results, model)?;

        // Determine output directory
        let eval_results_dir = if let Some(first_run) = all_runs_results.first() {
            if let Some(first_suite) = first_run.suites.first() {
                if let Some(first_eval) = first_suite.evaluations.first() {
                    let eval_path = EvalRunner::path_for_eval(
                        &model,
                        &BenchEval {
                            selector: format!("{}:{}", first_suite.name, first_eval.name),
                            post_process_cmd: None,
                            parallel_safe: true,
                        },
                        "0".to_string(),
                    );
                    if let Some(parent) = eval_path.parent() {
                        if let Some(benchmark_dir) = parent.parent() {
                            Some(benchmark_dir.join("eval-results"))
                        } else {
                            None
                        }
                    } else {
                        None
                    }
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
            fs::create_dir_all(&dir)?;
            Self::generate_eval_csvs(&df, &dir)?;
            Self::generate_aggregate_metrics(&df, &dir)?;
        }

        Ok(())
    }

    fn results_to_dataframe(
        results: &[BenchmarkResults],
        model: &BenchModel,
    ) -> anyhow::Result<DataFrame> {
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

        // Get all column names
        let columns: HashSet<String> = records
            .iter()
            .flat_map(|record| record.keys().cloned())
            .collect();

        // Create series for each column
        let series: Vec<Series> = columns
            .iter()
            .map(|column| {
                let values: Vec<String> = records
                    .iter()
                    .map(|record| record.get(column).cloned().unwrap_or_default())
                    .collect();
                Series::new(column, values)
            })
            .collect();

        DataFrame::new(series).context("Failed to create DataFrame")
    }

    fn run_benchmark(
        &mut self,
        model: &BenchModel,
        suites: HashMap<String, Vec<BenchEval>>,
        run_id: String,
    ) -> anyhow::Result<()> {
        let mut results_handles = HashMap::<String, Vec<Child>>::new();

        // Load environment variables from file if specified
        let mut envs = self.toolshim_envs();
        if let Some(env_file) = &self.config.env_file {
            let env_vars = self.load_env_file(env_file)?;
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
                    self.config.run_id = Some(run_id.clone());
                    self.config.evals = vec![(*eval_selector).clone()];
                    let cfg = self.config.to_string()?;
                    let handle = parallel_bench_cmd("exec-eval".to_string(), cfg, envs.clone());
                    results_handles.get_mut(suite).unwrap().push(handle);
                }
            }

            // Run non-parallel-safe evaluations sequentially
            for eval_selector in &sequential_evals {
                self.config.run_id = Some(run_id.clone());
                self.config.evals = vec![(*eval_selector).clone()];
                let cfg = self.config.to_string()?;
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
    ) -> anyhow::Result<BenchmarkResults> {
        let mut results = BenchmarkResults::new(model.provider.clone());

        let mut summary_path: Option<PathBuf> = None;

        for (suite, evals) in suites.iter() {
            let mut suite_result = SuiteResult::new(suite.clone());
            for eval_selector in evals {
                let mut eval_path =
                    EvalRunner::path_for_eval(&model, eval_selector, run_id.clone());
                eval_path.push(self.config.eval_result_filename.clone());
                let eval_result = serde_json::from_str(&read_to_string(&eval_path)?)?;
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

        let mut run_summary = PathBuf::new();
        run_summary.push(summary_path.clone().unwrap());
        run_summary.push(&self.config.run_summary_filename);

        let output_str = serde_json::to_string_pretty(&results)?;
        std::fs::write(run_summary, &output_str)?;

        Ok(results)
    }

    fn collect_evals_for_run(&self) -> HashMap<String, Vec<BenchEval>> {
        // convert suites map {suite_name => [eval_selector_str] to map suite_name => [BenchEval]
        let suites = self
            .config
            .evals
            .iter()
            .map(|eval| {
                EvaluationSuite::select(vec![eval.clone().selector])
                    .iter()
                    .map(|(suite, evals)| {
                        let bench_evals = evals
                            .iter()
                            .map(|suite_eval| {
                                let mut updated_eval = eval.clone();
                                updated_eval.selector = (*suite_eval).to_string();
                                updated_eval
                            })
                            .collect::<Vec<_>>();
                        (suite.clone(), bench_evals)
                    })
                    .collect()
            })
            .collect();
        union_hashmaps(suites)
    }

    fn toolshim_envs(&self) -> Vec<(String, String)> {
        // read tool-shim preference from config, set respective env vars accordingly
        let mut shim_envs: Vec<(String, String)> = Vec::new();
        if let Some(shim_opt) = &self.config.tool_shim {
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
        shim_envs
    }

    fn load_env_file(&self, path: &PathBuf) -> anyhow::Result<Vec<(String, String)>> {
        let file = std::fs::File::open(path)?;
        let reader = io::BufReader::new(file);
        let mut env_vars = Vec::new();

        for line in reader.lines() {
            let line = line?;
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
