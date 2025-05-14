use crate::errors::BenchError;
use chrono::Local;
use lazy_static::lazy_static;
use std::fmt;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;

/// Log levels for the benchmark runner
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
    /// Only critical errors
    Error = 0,
    /// Important warnings
    Warn = 1,
    /// Normal informational messages
    Info = 2,
    /// Debug information
    Debug = 3,
    /// Verbose tracing information
    Trace = 4,
}

impl fmt::Display for LogLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LogLevel::Error => write!(f, "ERROR"),
            LogLevel::Warn => write!(f, "WARN "),
            LogLevel::Info => write!(f, "INFO "),
            LogLevel::Debug => write!(f, "DEBUG"),
            LogLevel::Trace => write!(f, "TRACE"),
        }
    }
}

struct LoggerState {
    file_path: Option<PathBuf>,
    console_level: LogLevel,
    file_level: LogLevel,
}

lazy_static! {
    static ref LOGGER_STATE: Mutex<LoggerState> = Mutex::new(LoggerState {
        file_path: None,
        console_level: LogLevel::Info,
        file_level: LogLevel::Debug,
    });
}

/// Configure the logger with specified levels and optional log file
pub fn configure_logger(
    console_level: LogLevel,
    file_level: LogLevel,
    log_file: Option<PathBuf>,
) -> Result<(), BenchError> {
    let mut state = LOGGER_STATE.lock().unwrap();

    state.console_level = console_level;
    state.file_level = file_level;
    state.file_path = log_file;

    // If a log file is specified, try to create/open it to verify we can write to it
    if let Some(file_path) = &state.file_path {
        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .append(true)
            .open(file_path)
            .map_err(|e| BenchError::IoError(e))?;

        let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
        let header = format!(
            "[{}] [INFO ] Logging initialized at level {} (console), {} (file)\n",
            timestamp, state.console_level, state.file_level
        );

        let mut file = file;
        file.write_all(header.as_bytes())
            .map_err(|e| BenchError::IoError(e))?;
    }

    Ok(())
}

/// Get the current console log level
pub fn get_console_level() -> LogLevel {
    let state = LOGGER_STATE.lock().unwrap();
    state.console_level
}

/// Get the current file log level
pub fn get_file_level() -> LogLevel {
    let state = LOGGER_STATE.lock().unwrap();
    state.file_level
}

/// Log a message with the specified level
pub fn log(level: LogLevel, message: &str) {
    let state = LOGGER_STATE.lock().unwrap();
    let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let log_entry = format!("[{}] [{}] {}\n", timestamp, level, message);

    // Log to console if level is sufficient
    if level <= state.console_level {
        match level {
            LogLevel::Error => eprintln!("{}", log_entry.trim()),
            _ => println!("{}", log_entry.trim()),
        }
    }

    // Log to file if configured and level is sufficient
    if let Some(file_path) = &state.file_path {
        if level <= state.file_level {
            if let Ok(mut file) = OpenOptions::new()
                .write(true)
                .create(true)
                .append(true)
                .open(file_path)
            {
                let _ = file.write_all(log_entry.as_bytes());
            }
        }
    }
}

/// Log an error message
pub fn error(message: &str) {
    log(LogLevel::Error, message);
}

/// Log a warning message
pub fn warn(message: &str) {
    log(LogLevel::Warn, message);
}

/// Log an info message
pub fn info(message: &str) {
    log(LogLevel::Info, message);
}

/// Log a debug message
pub fn debug(message: &str) {
    log(LogLevel::Debug, message);
}

/// Log a trace message
pub fn trace(message: &str) {
    log(LogLevel::Trace, message);
}

/// Log an error with additional context
pub fn log_error<E: fmt::Display>(err: E, context: &str) {
    error(&format!("{}: {}", context, err));
}

/// Macro to log with file and line information
#[macro_export]
macro_rules! log_with_location {
    ($level:expr, $($arg:tt)*) => {
        $crate::logging::log($level, &format!("[{}:{}] {}", file!(), line!(), format!($($arg)*)))
    };
}

/// Macros for common log levels with location information
#[macro_export]
macro_rules! error_loc {
    ($($arg:tt)*) => { $crate::log_with_location!($crate::logging::LogLevel::Error, $($arg)*) };
}

#[macro_export]
macro_rules! warn_loc {
    ($($arg:tt)*) => { $crate::log_with_location!($crate::logging::LogLevel::Warn, $($arg)*) };
}

#[macro_export]
macro_rules! info_loc {
    ($($arg:tt)*) => { $crate::log_with_location!($crate::logging::LogLevel::Info, $($arg)*) };
}

#[macro_export]
macro_rules! debug_loc {
    ($($arg:tt)*) => { $crate::log_with_location!($crate::logging::LogLevel::Debug, $($arg)*) };
}

#[macro_export]
macro_rules! trace_loc {
    ($($arg:tt)*) => { $crate::log_with_location!($crate::logging::LogLevel::Trace, $($arg)*) };
}
