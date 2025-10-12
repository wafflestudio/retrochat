use anyhow::Result;
use std::env;
use std::path::PathBuf;
use tracing::Level;
use tracing_subscriber::{
    filter::LevelFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt, Layer, Registry,
};

use crate::env::logging as env_vars;

/// Simplified logging configuration for RetroChat
#[derive(Debug, Clone)]
pub struct LoggingConfig {
    /// Log level (error, warn, info, debug, trace)
    pub level: Level,
    /// Whether to log to stdout
    pub stdout: bool,
    /// Optional file path for logging
    pub file_path: Option<PathBuf>,
    /// Whether to use JSON format
    pub json_format: bool,
    /// Whether to use ANSI colors
    pub use_colors: bool,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: Level::INFO,
            stdout: true,
            file_path: None,
            json_format: false,
            use_colors: true,
        }
    }
}

impl LoggingConfig {
    /// Create a new logging config with reasonable defaults
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the log level
    pub fn with_level(mut self, level: Level) -> Self {
        self.level = level;
        self
    }

    /// Enable/disable stdout logging
    pub fn with_stdout(mut self, enabled: bool) -> Self {
        self.stdout = enabled;
        self
    }

    /// Set file path for logging
    pub fn with_file<P: Into<PathBuf>>(mut self, path: P) -> Self {
        self.file_path = Some(path.into());
        self
    }

    /// Enable/disable JSON format
    pub fn with_json_format(mut self, enabled: bool) -> Self {
        self.json_format = enabled;
        self
    }

    /// Enable/disable ANSI colors
    pub fn with_colors(mut self, enabled: bool) -> Self {
        self.use_colors = enabled;
        self
    }

    /// Create config from environment variables
    pub fn from_env() -> Self {
        let mut config = Self::default();

        // Set log level from environment
        if let Ok(level_str) = env::var(env_vars::LOG_LEVEL) {
            config.level = match level_str.to_lowercase().as_str() {
                "error" => Level::ERROR,
                "warn" => Level::WARN,
                "info" => Level::INFO,
                "debug" => Level::DEBUG,
                "trace" => Level::TRACE,
                _ => Level::INFO,
            };
        }

        // Set file path from environment
        if let Ok(file_path) = env::var(env_vars::LOG_FILE) {
            config.file_path = Some(PathBuf::from(file_path));
        }

        // Disable colors if RETROCHAT_NO_COLOR is set
        if env::var(env_vars::NO_COLOR).is_ok() {
            config.use_colors = false;
        }

        config
    }

    /// Create a development config with debug logging
    pub fn development() -> Self {
        Self {
            level: Level::DEBUG,
            stdout: true,
            file_path: None,
            json_format: false,
            use_colors: true,
        }
    }

    /// Create a production config with structured logging
    pub fn production() -> Self {
        Self {
            level: Level::INFO,
            stdout: false,
            file_path: Some(PathBuf::from("/var/log/retrochat/app.log")),
            json_format: true,
            use_colors: false,
        }
    }
}

/// Initialize logging with the given configuration
pub fn init_logging(config: LoggingConfig) -> Result<()> {
    let registry = Registry::default();

    // Simple implementation - just use stdout for now
    let layer = fmt::layer()
        .with_ansi(config.use_colors)
        .with_level(true)
        .with_target(true)
        .with_filter(LevelFilter::from_level(config.level));

    registry.with(layer).init();

    // Log initialization
    tracing::info!(
        level = ?config.level,
        stdout = config.stdout,
        file_path = ?config.file_path,
        json_format = config.json_format,
        "Logging initialized"
    );

    Ok(())
}

/// Initialize simple logging for development
pub fn init_simple() -> Result<()> {
    init_logging(LoggingConfig::development())
}

/// Initialize logging from environment variables
pub fn init_from_env() -> Result<()> {
    init_logging(LoggingConfig::from_env())
}

/// Log error with context
pub fn log_error<E: std::fmt::Display>(error: &E, context: &str) {
    tracing::error!(error = %error, context = context, "Error occurred");
}

/// Log error with detailed context and category
pub fn log_error_detailed<E: std::fmt::Display>(
    error: &E,
    context: &str,
    category: &str,
    operation: &str,
) {
    tracing::error!(
        error = %error,
        context = context,
        category = category,
        operation = operation,
        "Detailed error occurred"
    );
}

/// Log performance metrics
pub fn log_performance(operation: &str, duration_ms: u64, success: bool) {
    if success {
        tracing::info!(
            operation = operation,
            duration_ms = duration_ms,
            success = success,
            "Operation completed"
        );
    } else {
        tracing::warn!(
            operation = operation,
            duration_ms = duration_ms,
            success = success,
            "Operation failed"
        );
    }
}

/// Log audit events
pub fn log_audit(event: &str, user: Option<&str>, resource: Option<&str>, action: &str) {
    tracing::info!(
        event = event,
        user = user,
        resource = resource,
        action = action,
        audit = true,
        "Audit event"
    );
}
