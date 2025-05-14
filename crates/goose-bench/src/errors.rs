use polars::error::PolarsError;
use std::fmt;
use std::io;
use std::path::PathBuf;
use thiserror::Error;

/// Custom error types for the benchmark runner
#[derive(Error, Debug)]
pub enum BenchError {
    #[error("Failed to parse configuration: {0}")]
    ConfigError(String),

    #[error("Failed to run benchmark: {0}")]
    BenchmarkError(String),

    #[error("Failed to process results: {0}")]
    ResultsProcessingError(String),

    #[error("Failed to run evaluation: {0}")]
    EvaluationError(String),

    #[error("File not found: {0}")]
    FileNotFound(PathBuf),

    #[error("Failed to parse JSON: {0}")]
    JsonParseError(#[from] serde_json::Error),

    #[error("I/O error: {0}")]
    IoError(#[from] io::Error),

    #[error("DataFrame error: {0}")]
    DataFrameError(String),

    #[error("Subprocess error with status: {0}")]
    SubprocessError(i32),

    #[error("Thread error: {0}")]
    ThreadError(String),

    #[error("Environment error: {0}")]
    EnvironmentError(String),

    #[error("Tool shim error: {0}")]
    ToolShimError(String),

    #[error("Other error: {0}")]
    Other(String),
}

/// Result type for benchmark operations
pub type BenchResult<T> = Result<T, BenchError>;

/// Utility functions for working with BenchError
pub mod util {
    use super::*;
    use std::path::Path;

    /// Check if a file exists, returning a FileNotFound error if it doesn't
    pub fn ensure_file_exists<P: AsRef<Path>>(path: P) -> BenchResult<()> {
        let path_ref = path.as_ref();
        if !path_ref.exists() {
            return Err(BenchError::FileNotFound(path_ref.to_path_buf()));
        }
        if !path_ref.is_file() {
            return Err(BenchError::FileNotFound(path_ref.to_path_buf()));
        }
        Ok(())
    }

    /// Convert a generic error to a BenchError
    pub fn to_bench_error<E: fmt::Display>(e: E, context: &str) -> BenchError {
        BenchError::Other(format!("{}: {}", context, e))
    }
}

/// Implement From<anyhow::Error> for BenchError
impl From<anyhow::Error> for BenchError {
    fn from(err: anyhow::Error) -> Self {
        BenchError::Other(err.to_string())
    }
}

/// Implement From<polars::error::PolarsError> for BenchError
impl From<PolarsError> for BenchError {
    fn from(err: PolarsError) -> Self {
        BenchError::DataFrameError(err.to_string())
    }
}
