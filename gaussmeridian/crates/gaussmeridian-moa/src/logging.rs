use std::path::PathBuf;
use tracing::{Level, Subscriber};
use tracing_subscriber::{
    fmt::{self, time::UtcTime},
    EnvFilter,
    Layer,
    Registry,
    layer::SubscriberExt,
};
use tracing_appender::{
    rolling::{RollingFileAppender, Rotation},
    non_blocking::WorkerGuard,
};
use serde::{Serialize, Deserialize};
use thiserror::Error;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogConfig {
    pub level: LogLevel,
    pub file_logging: Option<FileLoggingConfig>,
    pub console_logging: bool,
    pub json_format: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileLoggingConfig {
    pub directory: PathBuf,
    pub prefix: String,
    pub rotation: LogRotation,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum LogLevel {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum LogRotation {
    Hourly,
    Daily,
    Never,
}

#[derive(Debug, Error)]
pub enum LoggingError {
    #[error("Failed to initialize file appender: {0}")]
    FileAppender(std::io::Error),
    
    #[error("Failed to set global subscriber: {0}")]
    SetGlobalDefault(String),
}

impl From<LogLevel> for Level {
    fn from(level: LogLevel) -> Self {
        match level {
            LogLevel::Error => Level::ERROR,
            LogLevel::Warn => Level::WARN,
            LogLevel::Info => Level::INFO,
            LogLevel::Debug => Level::DEBUG,
            LogLevel::Trace => Level::TRACE,
        }
    }
}

impl From<LogRotation> for Rotation {
    fn from(rotation: LogRotation) -> Self {
        match rotation {
            LogRotation::Hourly => Rotation::HOURLY,
            LogRotation::Daily => Rotation::DAILY,
            LogRotation::Never => Rotation::NEVER,
        }
    }
}

pub struct LogManager {
    _file_guard: Option<WorkerGuard>,
}

impl LogManager {
    pub fn init(config: LogConfig) -> Result<Self, LoggingError> {
        let env_filter = EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| EnvFilter::new("info"));

        let mut layers = Vec::new();

        // Configure console logging if enabled
        if config.console_logging {
            let console_layer = fmt::layer()
                .with_target(true)
                .with_thread_ids(true)
                .with_line_number(true)
                .with_timer(UtcTime::rfc_3339())
                .with_filter(env_filter.clone());

            if config.json_format {
                layers.push(console_layer.json().boxed());
            } else {
                layers.push(console_layer.boxed());
            }
        }

        // Configure file logging if enabled
        let file_guard = if let Some(file_config) = config.file_logging {
            let file_appender = RollingFileAppender::new(
                Rotation::from(file_config.rotation),
                file_config.directory,
                file_config.prefix,
            ).map_err(LoggingError::FileAppender)?;

            let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);
            
            let file_layer = fmt::layer()
                .with_target(true)
                .with_thread_ids(true)
                .with_line_number(true)
                .with_timer(UtcTime::rfc_3339())
                .with_writer(non_blocking)
                .with_filter(env_filter);

            if config.json_format {
                layers.push(file_layer.json().boxed());
            } else {
                layers.push(file_layer.boxed());
            }

            Some(guard)
        } else {
            None
        };

        // Create and set the subscriber
        let subscriber = Registry::default().with(layers);
        tracing::subscriber::set_global_default(subscriber)
            .map_err(|e| LoggingError::SetGlobalDefault(e.to_string()))?;

        Ok(Self {
            _file_guard: file_guard,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use std::fs;
    use tracing::{info, error, debug};

    #[test]
    fn test_console_logging() {
        let config = LogConfig {
            level: LogLevel::Debug,
            file_logging: None,
            console_logging: true,
            json_format: false,
        };

        let _log_manager = LogManager::init(config).unwrap();

        info!("Test info message");
        error!("Test error message");
        debug!("Test debug message");
    }

    #[test]
    fn test_file_logging() {
        let temp_dir = tempdir().unwrap();
        let config = LogConfig {
            level: LogLevel::Debug,
            file_logging: Some(FileLoggingConfig {
                directory: temp_dir.path().to_path_buf(),
                prefix: "test".to_string(),
                rotation: LogRotation::Never,
            }),
            console_logging: false,
            json_format: true,
        };

        let _log_manager = LogManager::init(config).unwrap();

        info!("Test info message");
        error!("Test error message");
        debug!("Test debug message");

        // Verify log file exists and contains content
        let log_files = fs::read_dir(temp_dir.path()).unwrap()
            .filter_map(Result::ok)
            .collect::<Vec<_>>();
        
        assert!(!log_files.is_empty());
    }
} 