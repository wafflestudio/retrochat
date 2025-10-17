use super::{ParsedTool, ToolData, ToolParser};
use crate::models::message::ToolUse;
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};

/// Bash tool parser
pub struct BashParser;

/// Structured data from Bash tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BashData {
    /// The command that was executed
    pub command: String,
    /// Optional description
    pub description: Option<String>,
    /// Optional timeout value
    pub timeout: Option<u64>,
}

impl ToolParser for BashParser {
    fn parse(&self, tool_use: &ToolUse) -> Result<ParsedTool> {
        let command = tool_use
            .input
            .get("command")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Bash tool missing 'command' field"))?
            .to_string();

        let description = tool_use
            .input
            .get("description")
            .and_then(|v| v.as_str())
            .map(String::from);

        let timeout = tool_use.input.get("timeout").and_then(|v| v.as_u64());

        let data = BashData {
            command,
            description,
            timeout,
        };

        Ok(ParsedTool::new(
            tool_use.name.clone(),
            ToolData::Bash(data),
            tool_use.input.clone(),
        ))
    }
}

impl BashData {
    /// Check if this is a potentially dangerous command
    pub fn is_dangerous(&self) -> bool {
        let dangerous_patterns = [
            "rm -rf",
            "rm -r /",
            "mkfs",
            "dd if=",
            "> /dev/",
            "mv /* ",
            "chmod -R 777",
            "wget", // Might be dangerous depending on context
            "curl", // Might be dangerous depending on context
        ];

        dangerous_patterns
            .iter()
            .any(|pattern| self.command.contains(pattern))
    }

    /// Extract the base command (first word)
    pub fn base_command(&self) -> &str {
        self.command
            .split_whitespace()
            .next()
            .unwrap_or(&self.command)
    }

    /// Check if this command modifies the filesystem
    pub fn is_mutation(&self) -> bool {
        let mutation_commands = ["rm", "mv", "cp", "mkdir", "touch", "rmdir", "ln"];
        mutation_commands.contains(&self.base_command())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn create_bash_tool_use(command: &str) -> ToolUse {
        ToolUse {
            id: "test_id".to_string(),
            name: "Bash".to_string(),
            input: json!({
                "command": command,
                "description": "Test command"
            }),
            raw: json!({}),
        }
    }

    #[test]
    fn test_bash_parser() {
        let parser = BashParser;
        let tool_use = create_bash_tool_use("ls -la");

        let result = parser.parse(&tool_use);
        assert!(result.is_ok());

        let parsed = result.unwrap();
        assert_eq!(parsed.tool_name, "Bash");

        if let ToolData::Bash(data) = parsed.data {
            assert_eq!(data.command, "ls -la");
            assert_eq!(data.description, Some("Test command".to_string()));
        } else {
            panic!("Expected BashData");
        }
    }

    #[test]
    fn test_bash_data_base_command() {
        let data = BashData {
            command: "ls -la /tmp".to_string(),
            description: None,
            timeout: None,
        };

        assert_eq!(data.base_command(), "ls");
    }

    #[test]
    fn test_bash_data_is_dangerous() {
        let dangerous = BashData {
            command: "rm -rf /".to_string(),
            description: None,
            timeout: None,
        };
        assert!(dangerous.is_dangerous());

        let safe = BashData {
            command: "ls -la".to_string(),
            description: None,
            timeout: None,
        };
        assert!(!safe.is_dangerous());
    }

    #[test]
    fn test_bash_data_is_mutation() {
        let mutation = BashData {
            command: "rm file.txt".to_string(),
            description: None,
            timeout: None,
        };
        assert!(mutation.is_mutation());

        let read_only = BashData {
            command: "ls -la".to_string(),
            description: None,
            timeout: None,
        };
        assert!(!read_only.is_mutation());
    }
}
