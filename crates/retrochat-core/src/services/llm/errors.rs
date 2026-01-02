//! LLM error types for multi-provider support
//!
//! This module provides provider-agnostic error types that can wrap
//! provider-specific errors.

use thiserror::Error;

/// Provider-agnostic LLM errors
#[derive(Debug, Error)]
pub enum LlmError {
    #[error("Configuration error: {message}")]
    ConfigurationError { message: String },

    #[error("Authentication failed: {message}")]
    AuthenticationFailed { message: String },

    #[error("Rate limit exceeded: {message}")]
    RateLimitExceeded { message: String },

    #[error("Request timeout after {timeout_secs}s")]
    Timeout { timeout_secs: u64 },

    #[error("Network error: {message}")]
    NetworkError { message: String },

    #[error("Invalid request: {message}")]
    InvalidRequest { message: String },

    #[error("Content blocked by safety filters")]
    ContentBlocked,

    #[error("Quota exceeded: {message}")]
    QuotaExceeded { message: String },

    #[error("Server error: {message}")]
    ServerError { message: String },

    #[error("Parse error: {message}")]
    ParseError { message: String },

    #[error("Invalid response: {message}")]
    InvalidResponse { message: String },

    #[error("CLI execution failed: {message}")]
    CliExecutionError { message: String },

    #[error("CLI binary not found: {path}")]
    CliBinaryNotFound { path: String },

    #[error("Provider unavailable: {message}")]
    ProviderUnavailable { message: String },
}

impl LlmError {
    /// Check if this error is potentially retryable
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            LlmError::RateLimitExceeded { .. }
                | LlmError::Timeout { .. }
                | LlmError::NetworkError { .. }
                | LlmError::ServerError { .. }
        )
    }

    /// Get suggested retry delay in seconds
    pub fn retry_after_secs(&self) -> Option<u64> {
        match self {
            LlmError::RateLimitExceeded { .. } => Some(60),
            LlmError::Timeout { .. } => Some(5),
            LlmError::ServerError { .. } => Some(30),
            _ => None,
        }
    }

    /// Convert to user-friendly message
    pub fn user_message(&self) -> String {
        match self {
            LlmError::ConfigurationError { message } => {
                format!("Configuration error: {message}")
            }
            LlmError::AuthenticationFailed { .. } => {
                "Authentication failed. Please check your API key or CLI setup.".to_string()
            }
            LlmError::RateLimitExceeded { .. } => {
                "Rate limit exceeded. Please wait a moment and try again.".to_string()
            }
            LlmError::Timeout { timeout_secs } => {
                format!("Request timed out after {timeout_secs} seconds.")
            }
            LlmError::NetworkError { .. } => {
                "Network connection error. Please check your internet connection.".to_string()
            }
            LlmError::ContentBlocked => {
                "Content was blocked by safety filters. Try rephrasing your request.".to_string()
            }
            LlmError::QuotaExceeded { .. } => {
                "API quota exceeded. Please check your usage limits.".to_string()
            }
            LlmError::ServerError { .. } => {
                "Server is experiencing issues. Please try again later.".to_string()
            }
            LlmError::CliExecutionError { message } => {
                format!("CLI execution failed: {message}")
            }
            LlmError::CliBinaryNotFound { path } => {
                format!(
                    "CLI binary not found at: {path}. Please ensure it's installed and in PATH."
                )
            }
            LlmError::ProviderUnavailable { message } => {
                format!("Provider unavailable: {message}")
            }
            LlmError::ParseError { .. } => "Error parsing response. Please try again.".to_string(),
            LlmError::InvalidResponse { .. } => {
                "Received invalid response. Please try again.".to_string()
            }
            LlmError::InvalidRequest { message } => {
                format!("Invalid request: {message}")
            }
        }
    }
}

// Conversion from GoogleAiError to LlmError
impl From<crate::services::google_ai::GoogleAiError> for LlmError {
    fn from(err: crate::services::google_ai::GoogleAiError) -> Self {
        use crate::services::google_ai::GoogleAiError;
        match err {
            GoogleAiError::AuthenticationFailed { message } => {
                LlmError::AuthenticationFailed { message }
            }
            GoogleAiError::RateLimitExceeded { message } => LlmError::RateLimitExceeded { message },
            GoogleAiError::Timeout { timeout_ms } => LlmError::Timeout {
                timeout_secs: timeout_ms / 1000,
            },
            GoogleAiError::NetworkError { source } => LlmError::NetworkError {
                message: source.to_string(),
            },
            GoogleAiError::InvalidRequest { message } => LlmError::InvalidRequest { message },
            GoogleAiError::ContentBlocked => LlmError::ContentBlocked,
            GoogleAiError::QuotaExceeded { message } => LlmError::QuotaExceeded { message },
            GoogleAiError::ServerError { message, .. } => LlmError::ServerError { message },
            GoogleAiError::ParseError { message } => LlmError::ParseError { message },
            GoogleAiError::InvalidResponse { message } => LlmError::InvalidResponse { message },
            GoogleAiError::ServiceUnavailable { message } => {
                LlmError::ProviderUnavailable { message }
            }
            GoogleAiError::ConfigurationError { message } => {
                LlmError::ConfigurationError { message }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_retryable() {
        assert!(LlmError::RateLimitExceeded {
            message: "test".to_string()
        }
        .is_retryable());
        assert!(LlmError::Timeout { timeout_secs: 30 }.is_retryable());
        assert!(LlmError::NetworkError {
            message: "test".to_string()
        }
        .is_retryable());
        assert!(LlmError::ServerError {
            message: "test".to_string()
        }
        .is_retryable());

        assert!(!LlmError::ConfigurationError {
            message: "test".to_string()
        }
        .is_retryable());
        assert!(!LlmError::AuthenticationFailed {
            message: "test".to_string()
        }
        .is_retryable());
        assert!(!LlmError::ContentBlocked.is_retryable());
    }

    #[test]
    fn test_retry_after_secs() {
        assert_eq!(
            LlmError::RateLimitExceeded {
                message: "test".to_string()
            }
            .retry_after_secs(),
            Some(60)
        );
        assert_eq!(
            LlmError::Timeout { timeout_secs: 30 }.retry_after_secs(),
            Some(5)
        );
        assert_eq!(
            LlmError::ServerError {
                message: "test".to_string()
            }
            .retry_after_secs(),
            Some(30)
        );
        assert_eq!(LlmError::ContentBlocked.retry_after_secs(), None);
    }

    #[test]
    fn test_user_message() {
        let error = LlmError::CliBinaryNotFound {
            path: "/usr/bin/claude".to_string(),
        };
        assert!(error.user_message().contains("/usr/bin/claude"));
        assert!(error.user_message().contains("not found"));
    }
}
