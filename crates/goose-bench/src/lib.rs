pub mod bench_config;
pub mod bench_session;
pub mod bench_work_dir;
pub mod config_manager;
pub mod dataframe_handler;
pub mod error_capture;
pub mod errors;
pub mod eval_suites;
pub mod io_utils;
pub mod logging;
pub mod reporting;
pub mod runners;
pub mod utilities;

// Re-export main components for easier use
pub use config_manager::ConfigManager;
pub use dataframe_handler::DataFrameHandler;
pub use errors::{BenchError, BenchResult};
pub use runners::benchmark_runner::BenchmarkRunner;
