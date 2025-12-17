use thiserror::Error;

/// Custom error types for RetroChat application
#[derive(Error, Debug)]
pub enum RetroChatError {
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON parsing error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("File parsing error: {message}")]
    FileParsing { message: String },

    #[error("Invalid configuration: {message}")]
    InvalidConfig { message: String },

    #[error("Import error: {message}")]
    Import { message: String },

    #[error("Export error: {message}")]
    Export { message: String },

    #[error("TUI error: {message}")]
    Tui { message: String },

    #[error("Service error: {message}")]
    Service { message: String },

    #[error("Authentication error: {message}")]
    Auth { message: String },

    #[cfg(feature = "reqwest")]
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    #[error("UUID parsing error: {0}")]
    Uuid(#[from] uuid::Error),

    #[error("Time parsing error: {0}")]
    Time(#[from] chrono::ParseError),

    #[error("Task join error: {0}")]
    TaskJoin(#[from] tokio::task::JoinError),

    #[error("Channel send error")]
    ChannelSend,

    #[error("Channel receive error")]
    ChannelReceive,

    #[error("Validation error: {field}: {message}")]
    Validation { field: String, message: String },

    #[error("Not found: {resource}")]
    NotFound { resource: String },

    #[error("Already exists: {resource}")]
    AlreadyExists { resource: String },

    #[error("Permission denied: {action}")]
    PermissionDenied { action: String },

    #[error("Rate limit exceeded: {limit}")]
    RateLimit { limit: String },

    #[error("External service error: {service}: {message}")]
    ExternalService { service: String, message: String },

    #[error("Unknown error: {message}")]
    Unknown { message: String },
}

impl RetroChatError {
    /// Create a file parsing error
    pub fn file_parsing<S: Into<String>>(message: S) -> Self {
        Self::FileParsing {
            message: message.into(),
        }
    }

    /// Create an invalid configuration error
    pub fn invalid_config<S: Into<String>>(message: S) -> Self {
        Self::InvalidConfig {
            message: message.into(),
        }
    }

    /// Create an import error
    pub fn import<S: Into<String>>(message: S) -> Self {
        Self::Import {
            message: message.into(),
        }
    }

    /// Create an export error
    pub fn export<S: Into<String>>(message: S) -> Self {
        Self::Export {
            message: message.into(),
        }
    }

    /// Create a TUI error
    pub fn tui<S: Into<String>>(message: S) -> Self {
        Self::Tui {
            message: message.into(),
        }
    }

    /// Create a service error
    pub fn service<S: Into<String>>(message: S) -> Self {
        Self::Service {
            message: message.into(),
        }
    }

    /// Create an authentication error
    pub fn auth<S: Into<String>>(message: S) -> Self {
        Self::Auth {
            message: message.into(),
        }
    }

    /// Create a validation error
    pub fn validation<S: Into<String>>(field: S, message: S) -> Self {
        Self::Validation {
            field: field.into(),
            message: message.into(),
        }
    }

    /// Create a not found error
    pub fn not_found<S: Into<String>>(resource: S) -> Self {
        Self::NotFound {
            resource: resource.into(),
        }
    }

    /// Create an already exists error
    pub fn already_exists<S: Into<String>>(resource: S) -> Self {
        Self::AlreadyExists {
            resource: resource.into(),
        }
    }

    /// Create a permission denied error
    pub fn permission_denied<S: Into<String>>(action: S) -> Self {
        Self::PermissionDenied {
            action: action.into(),
        }
    }

    /// Create a rate limit error
    pub fn rate_limit<S: Into<String>>(limit: S) -> Self {
        Self::RateLimit {
            limit: limit.into(),
        }
    }

    /// Create an external service error
    pub fn external_service<S: Into<String>>(service: S, message: S) -> Self {
        Self::ExternalService {
            service: service.into(),
            message: message.into(),
        }
    }

    /// Create an unknown error
    pub fn unknown<S: Into<String>>(message: S) -> Self {
        Self::Unknown {
            message: message.into(),
        }
    }

    /// Check if this error is retryable
    pub fn is_retryable(&self) -> bool {
        match self {
            #[cfg(feature = "reqwest")]
            RetroChatError::Network(_) => true,
            RetroChatError::ExternalService { .. } => true,
            RetroChatError::RateLimit { .. } => true,
            RetroChatError::Io(_) => true,
            _ => false,
        }
    }

    /// Get error category for logging and metrics
    pub fn category(&self) -> &'static str {
        match self {
            RetroChatError::Database(_) => "database",
            RetroChatError::Io(_) => "io",
            RetroChatError::Json(_) => "json",
            RetroChatError::FileParsing { .. } => "file_parsing",
            RetroChatError::InvalidConfig { .. } => "config",
            RetroChatError::Import { .. } => "import",
            RetroChatError::Export { .. } => "export",
            RetroChatError::Tui { .. } => "tui",
            RetroChatError::Service { .. } => "service",
            RetroChatError::Auth { .. } => "auth",
            #[cfg(feature = "reqwest")]
            RetroChatError::Network(_) => "network",
            RetroChatError::Uuid(_) => "uuid",
            RetroChatError::Time(_) => "time",
            RetroChatError::TaskJoin(_) => "task",
            RetroChatError::ChannelSend => "channel",
            RetroChatError::ChannelReceive => "channel",
            RetroChatError::Validation { .. } => "validation",
            RetroChatError::NotFound { .. } => "not_found",
            RetroChatError::AlreadyExists { .. } => "already_exists",
            RetroChatError::PermissionDenied { .. } => "permission",
            RetroChatError::RateLimit { .. } => "rate_limit",
            RetroChatError::ExternalService { .. } => "external",
            RetroChatError::Unknown { .. } => "unknown",
        }
    }
}

/// Convert anyhow::Error to RetroChatError
impl From<anyhow::Error> for RetroChatError {
    fn from(err: anyhow::Error) -> Self {
        // Try to downcast to known error types first
        // Try to convert to specific errors
        // Note: anyhow::Error doesn't support easy downcasting, so we'll parse the error message

        // Fall back to unknown error
        RetroChatError::Unknown {
            message: err.to_string(),
        }
    }
}

/// Convert channel send errors
impl<T> From<tokio::sync::mpsc::error::SendError<T>> for RetroChatError {
    fn from(_: tokio::sync::mpsc::error::SendError<T>) -> Self {
        RetroChatError::ChannelSend
    }
}

/// Convert channel receive errors
impl From<tokio::sync::oneshot::error::RecvError> for RetroChatError {
    fn from(_: tokio::sync::oneshot::error::RecvError) -> Self {
        RetroChatError::ChannelReceive
    }
}

/// Result type alias for RetroChat
pub type Result<T> = std::result::Result<T, RetroChatError>;
