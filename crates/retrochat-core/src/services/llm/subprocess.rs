//! Subprocess execution utilities for CLI-based LLM providers
//!
//! This module provides utilities for running CLI tools like Claude Code
//! and Gemini CLI as subprocesses.

use std::process::Stdio;
use std::time::Duration;
use tokio::process::Command;
use tokio::time::timeout;

use super::errors::LlmError;

/// Result of subprocess execution
#[derive(Debug)]
pub struct SubprocessResult {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
}

/// Run a CLI command with timeout and capture output
///
/// # Arguments
/// * `command` - The command to execute (e.g., "claude", "gemini")
/// * `args` - Command line arguments
/// * `timeout_secs` - Timeout in seconds
///
/// # Returns
/// * `Ok(SubprocessResult)` - Command output and exit code
/// * `Err(LlmError)` - If command fails to execute or times out
pub async fn run_cli_command(
    command: &str,
    args: &[&str],
    timeout_secs: u64,
) -> Result<SubprocessResult, LlmError> {
    let mut cmd = Command::new(command);
    cmd.args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);

    let child = cmd.spawn().map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            LlmError::CliBinaryNotFound {
                path: command.to_string(),
            }
        } else {
            LlmError::CliExecutionError {
                message: format!("Failed to spawn process: {e}"),
            }
        }
    })?;

    // Wait for completion with timeout
    let output = timeout(Duration::from_secs(timeout_secs), child.wait_with_output())
        .await
        .map_err(|_| LlmError::Timeout { timeout_secs })?
        .map_err(|e| LlmError::CliExecutionError {
            message: format!("Process execution failed: {e}"),
        })?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let exit_code = output.status.code().unwrap_or(-1);

    Ok(SubprocessResult {
        stdout,
        stderr,
        exit_code,
    })
}

/// Check if a CLI binary exists and is executable
///
/// # Arguments
/// * `command` - The command name to check (e.g., "claude", "gemini")
///
/// # Returns
/// * `true` if the command is found in PATH
/// * `false` otherwise
pub async fn check_cli_available(command: &str) -> bool {
    // Use 'which' on Unix-like systems
    #[cfg(unix)]
    {
        Command::new("which")
            .arg(command)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .await
            .map(|s| s.success())
            .unwrap_or(false)
    }

    // Use 'where' on Windows
    #[cfg(windows)]
    {
        Command::new("where")
            .arg(command)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .await
            .map(|s| s.success())
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_check_cli_available_echo() {
        // 'echo' should be available on all systems
        #[cfg(unix)]
        {
            let result = check_cli_available("echo").await;
            assert!(result);
        }
    }

    #[tokio::test]
    async fn test_check_cli_available_nonexistent() {
        let result = check_cli_available("nonexistent_command_12345").await;
        assert!(!result);
    }

    #[tokio::test]
    async fn test_run_cli_command_success() {
        #[cfg(unix)]
        {
            let result = run_cli_command("echo", &["hello"], 10).await;
            assert!(result.is_ok());
            let output = result.unwrap();
            assert_eq!(output.exit_code, 0);
            assert!(output.stdout.trim() == "hello");
        }
    }

    #[tokio::test]
    async fn test_run_cli_command_not_found() {
        let result = run_cli_command("nonexistent_command_12345", &[], 10).await;
        assert!(result.is_err());
        if let Err(LlmError::CliBinaryNotFound { path }) = result {
            assert_eq!(path, "nonexistent_command_12345");
        } else {
            panic!("Expected CliBinaryNotFound error");
        }
    }

    #[tokio::test]
    async fn test_run_cli_command_timeout() {
        #[cfg(unix)]
        {
            // Use a command that takes longer than timeout
            let result = run_cli_command("sleep", &["10"], 1).await;
            assert!(result.is_err());
            if let Err(LlmError::Timeout { timeout_secs }) = result {
                assert_eq!(timeout_secs, 1);
            } else {
                panic!("Expected Timeout error");
            }
        }
    }
}
