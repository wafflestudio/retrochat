use serde::{Deserialize, Serialize};

/// Bash-specific metadata for tool operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BashMetadata {
    /// Type of bash operation (e.g., "GitAdd", "Copy", "Delete", "Build")
    pub operation_type: String,
    
    /// The actual bash command that was executed
    pub command: String,
    
    /// Working directory where the command was executed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub working_directory: Option<String>,
    
    /// Exit code of the command (0 = success, non-zero = error)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
    
    /// Standard output from the command
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stdout: Option<String>,
    
    /// Standard error from the command
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stderr: Option<String>,
    
    /// Whether the command is considered dangerous (e.g., rm -rf, chmod 777)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_dangerous: Option<bool>,
}

impl BashMetadata {
    pub fn new(operation_type: String, command: String) -> Self {
        Self {
            operation_type,
            command,
            working_directory: None,
            exit_code: None,
            stdout: None,
            stderr: None,
            is_dangerous: None,
        }
    }

    pub fn with_working_directory(mut self, working_directory: String) -> Self {
        self.working_directory = Some(working_directory);
        self
    }

    pub fn with_exit_code(mut self, exit_code: i32) -> Self {
        self.exit_code = Some(exit_code);
        self
    }

    pub fn with_stdout(mut self, stdout: String) -> Self {
        self.stdout = Some(stdout);
        self
    }

    pub fn with_stderr(mut self, stderr: String) -> Self {
        self.stderr = Some(stderr);
        self
    }

    pub fn with_dangerous_flag(mut self, is_dangerous: bool) -> Self {
        self.is_dangerous = Some(is_dangerous);
        self
    }
}