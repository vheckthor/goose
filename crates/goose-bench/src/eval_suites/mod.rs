mod core;
mod evaluation;
mod factory;
mod list_files;

pub use evaluation::*;
pub use factory::{register_evaluation, EvaluationSuiteFactory};