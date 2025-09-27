use thiserror::Error;

#[derive(Debug, Error)]
pub enum GoogleAiError {
    #[error("Authentication failed: {message}")]
    AuthenticationFailed { message: String },

    #[error("Rate limit exceeded: {message}")]
    RateLimitExceeded { message: String },

    #[error("Request timeout after {timeout_ms}ms")]
    Timeout { timeout_ms: u64 },

    #[error("Network error: {source}")]
    NetworkError { source: reqwest::Error },

    #[error("Invalid request: {message}")]
    InvalidRequest { message: String },

    #[error("Content blocked by safety filters")]
    ContentBlocked,

    #[error("Quota exceeded: {message}")]
    QuotaExceeded { message: String },

    #[error("Server error: {status} - {message}")]
    ServerError { status: u16, message: String },

    #[error("Parse error: {message}")]
    ParseError { message: String },

    #[error("Invalid response: {message}")]
    InvalidResponse { message: String },

    #[error("Service unavailable: {message}")]
    ServiceUnavailable { message: String },

    #[error("Configuration error: {message}")]
    ConfigurationError { message: String },
}

impl GoogleAiError {
    pub fn is_retryable(&self) -> bool {
        match self {
            GoogleAiError::RateLimitExceeded { .. } => true,
            GoogleAiError::Timeout { .. } => true,
            GoogleAiError::NetworkError { .. } => true,
            GoogleAiError::ServerError { status, .. } => *status >= 500,
            GoogleAiError::ServiceUnavailable { .. } => true,
            _ => false,
        }
    }

    pub fn is_authentication_error(&self) -> bool {
        matches!(self, GoogleAiError::AuthenticationFailed { .. })
    }

    pub fn is_rate_limit_error(&self) -> bool {
        matches!(self, GoogleAiError::RateLimitExceeded { .. })
    }

    pub fn is_timeout_error(&self) -> bool {
        matches!(self, GoogleAiError::Timeout { .. })
    }

    pub fn is_network_error(&self) -> bool {
        matches!(self, GoogleAiError::NetworkError { .. })
    }

    pub fn is_quota_error(&self) -> bool {
        matches!(self, GoogleAiError::QuotaExceeded { .. })
    }

    pub fn is_content_blocked(&self) -> bool {
        matches!(self, GoogleAiError::ContentBlocked)
    }

    pub fn is_parse_error(&self) -> bool {
        matches!(self, GoogleAiError::ParseError { .. })
    }

    pub fn is_invalid_response_error(&self) -> bool {
        matches!(self, GoogleAiError::InvalidResponse { .. })
    }

    pub fn is_server_error(&self) -> bool {
        matches!(self, GoogleAiError::ServerError { .. })
    }

    pub fn retry_after_seconds(&self) -> Option<u64> {
        match self {
            GoogleAiError::RateLimitExceeded { .. } => Some(60), // Wait 60 seconds for rate limits
            GoogleAiError::Timeout { .. } => Some(5),            // Wait 5 seconds for timeouts
            GoogleAiError::ServerError { .. } => Some(30), // Wait 30 seconds for server errors
            GoogleAiError::ServiceUnavailable { .. } => Some(120), // Wait 2 minutes for service issues
            _ => None,
        }
    }

    pub fn user_message(&self) -> String {
        match self {
            GoogleAiError::AuthenticationFailed { .. } => {
                "Google AI API authentication failed. Please check your API key.".to_string()
            }
            GoogleAiError::RateLimitExceeded { .. } => {
                "Google AI API rate limit exceeded. Please wait a moment and try again.".to_string()
            }
            GoogleAiError::Timeout { .. } => {
                "Request timed out. The analysis may be taking longer than expected.".to_string()
            }
            GoogleAiError::NetworkError { .. } => {
                "Network connection error. Please check your internet connection.".to_string()
            }
            GoogleAiError::ContentBlocked => {
                "Content was blocked by safety filters. Try rephrasing your request.".to_string()
            }
            GoogleAiError::QuotaExceeded { .. } => {
                "API quota exceeded. Please check your Google AI usage limits.".to_string()
            }
            GoogleAiError::ServerError { .. } => {
                "Google AI service is experiencing issues. Please try again later.".to_string()
            }
            GoogleAiError::ServiceUnavailable { .. } => {
                "Google AI service is temporarily unavailable. Please try again later.".to_string()
            }
            GoogleAiError::InvalidRequest { message } => {
                format!("Invalid request: {message}")
            }
            GoogleAiError::ParseError { .. } => {
                "Error parsing Google AI response. Please try again.".to_string()
            }
            GoogleAiError::InvalidResponse { .. } => {
                "Received invalid response from Google AI. Please try again.".to_string()
            }
            GoogleAiError::ConfigurationError { message } => {
                format!("Configuration error: {message}")
            }
        }
    }

    pub fn from_reqwest_error(error: reqwest::Error) -> Self {
        if error.is_timeout() {
            GoogleAiError::Timeout {
                timeout_ms: 30000, // Default timeout value
            }
        } else if error.is_connect() {
            GoogleAiError::NetworkError { source: error }
        } else if let Some(status) = error.status() {
            let status_code = status.as_u16();
            let message = error.to_string();

            match status_code {
                401 => GoogleAiError::AuthenticationFailed { message },
                403 => GoogleAiError::QuotaExceeded { message },
                429 => GoogleAiError::RateLimitExceeded { message },
                500..=599 => GoogleAiError::ServerError {
                    status: status_code,
                    message,
                },
                _ => GoogleAiError::InvalidRequest { message },
            }
        } else {
            GoogleAiError::NetworkError { source: error }
        }
    }

    pub fn from_status_and_body(status: reqwest::StatusCode, body: &str) -> Self {
        let status_code = status.as_u16();

        // Try to parse error details from response body
        let error_message =
            if let Ok(error_response) = serde_json::from_str::<serde_json::Value>(body) {
                error_response
                    .get("error")
                    .and_then(|e| e.get("message"))
                    .and_then(|m| m.as_str())
                    .unwrap_or(body)
                    .to_string()
            } else {
                body.to_string()
            };

        match status_code {
            400 => GoogleAiError::InvalidRequest {
                message: error_message,
            },
            401 => GoogleAiError::AuthenticationFailed {
                message: error_message,
            },
            403 => {
                if error_message.to_lowercase().contains("quota") {
                    GoogleAiError::QuotaExceeded {
                        message: error_message,
                    }
                } else {
                    GoogleAiError::AuthenticationFailed {
                        message: error_message,
                    }
                }
            }
            429 => GoogleAiError::RateLimitExceeded {
                message: error_message,
            },
            503 => GoogleAiError::ServiceUnavailable {
                message: error_message,
            },
            500..=599 => GoogleAiError::ServerError {
                status: status_code,
                message: error_message,
            },
            _ => GoogleAiError::InvalidRequest {
                message: format!("HTTP {status_code}: {error_message}"),
            },
        }
    }
}

#[derive(Debug, Error)]
pub enum RetryError {
    #[error("Maximum retry attempts exceeded")]
    MaxAttemptsExceeded,

    #[error("Retry timeout exceeded")]
    TimeoutExceeded,

    #[error("Non-retryable error: {source}")]
    NonRetryable { source: GoogleAiError },
}

impl From<GoogleAiError> for RetryError {
    fn from(error: GoogleAiError) -> Self {
        if error.is_retryable() {
            RetryError::MaxAttemptsExceeded
        } else {
            RetryError::NonRetryable { source: error }
        }
    }
}
