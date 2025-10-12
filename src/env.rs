//! Environment variable constants used throughout the application
//!
//! This module centralizes all environment variable names to ensure consistency
//! and make it easier to manage configuration across the codebase.

/// Logging configuration
pub mod logging {
    /// Log level configuration (e.g., "debug", "info", "warn", "error")
    pub const LOG_LEVEL: &str = "RETROCHAT_LOG_LEVEL";

    /// Log file path for file-based logging
    pub const LOG_FILE: &str = "RETROCHAT_LOG_FILE";

    /// Disable colored output (follows the NO_COLOR standard)
    pub const NO_COLOR: &str = "NO_COLOR";
}

/// Provider directory configuration
pub mod providers {
    /// Claude Code chat history directories (colon-separated)
    pub const CLAUDE_DIRS: &str = "RETROCHAT_CLAUDE_DIRS";

    /// Gemini CLI chat history directories (colon-separated)
    pub const GEMINI_DIRS: &str = "RETROCHAT_GEMINI_DIRS";

    /// Cursor Agent chat history directories (colon-separated)
    pub const CURSOR_DIRS: &str = "RETROCHAT_CURSOR_DIRS";

    /// Codex chat history directories (colon-separated)
    pub const CODEX_DIRS: &str = "RETROCHAT_CODEX_DIRS";
}

/// External API configuration
pub mod apis {
    /// Google AI API key for retrospection analysis
    pub const GOOGLE_AI_API_KEY: &str = "GOOGLE_AI_API_KEY";
}

/// System environment variables
pub mod system {
    /// Home directory path
    pub const HOME: &str = "HOME";
}

/// Retrospection operation configuration (from specs)
pub mod retrospection {
    /// Default timeout for operations (seconds)
    pub const TIMEOUT: &str = "RETROCHAT_TIMEOUT";

    /// Maximum concurrent analysis operations
    pub const CONCURRENT: &str = "RETROCHAT_CONCURRENT";
}
