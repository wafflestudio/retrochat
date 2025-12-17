use super::{ParsedTool, ToolData, ToolParser};
use crate::models::message::ToolUse;
use anyhow::{anyhow, Result};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Bash tool parser with file operation tracking
pub struct BashParser;

/// Structured data from Bash tool with file operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BashData {
    /// The command that was executed
    pub command: String,
    /// Optional description
    pub description: Option<String>,
    /// Optional timeout value
    pub timeout: Option<u64>,
    /// File operations detected from the command
    pub file_operations: Vec<FileOperation>,
}

/// A file operation detected from a bash command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileOperation {
    /// Type of file operation
    pub operation_type: FileOperationType,
    /// File paths affected by this operation
    pub file_paths: Vec<String>,
}

/// Types of file operations that can be detected from bash commands
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum FileOperationType {
    // Git operations
    GitAdd,
    GitCommit,
    GitCheckout,
    GitMerge,
    GitMove,
    GitRemove,

    // File system operations
    Create, // mkdir, touch
    Copy,   // cp
    Move,   // mv
    Delete, // rm

    // Build & package operations
    Build,         // cargo build, npm run build, etc.
    Format,        // cargo fmt, prettier, etc.
    PackageAdd,    // cargo add, npm install
    PackageRemove, // cargo remove, npm uninstall

    // Other operations
    Modify, // in-place edit commands
    Search, // find, grep operations that might affect files
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

        let file_operations = Self::extract_file_operations(&command);

        let data = BashData {
            command,
            description,
            timeout,
            file_operations,
        };

        Ok(ParsedTool::new(
            tool_use.name.clone(),
            ToolData::Bash(data),
            tool_use.input.clone(),
        ))
    }
}

impl BashParser {
    /// Extract file operations from a bash command
    fn extract_file_operations(command: &str) -> Vec<FileOperation> {
        let mut operations = Vec::new();

        // Git operations
        if let Some(op) = Self::parse_git_command(command) {
            operations.push(op);
        }

        // File system operations
        operations.extend(Self::parse_fs_commands(command));

        // Build/format/package operations
        operations.extend(Self::parse_tooling_commands(command));

        // Other operations
        operations.extend(Self::parse_other_commands(command));

        operations
    }

    /// Parse git commands
    fn parse_git_command(command: &str) -> Option<FileOperation> {
        lazy_static::lazy_static! {
            static ref GIT_ADD: Regex = Regex::new(r"git\s+add\s+(.+)").unwrap();
            static ref GIT_COMMIT: Regex = Regex::new(r"git\s+commit").unwrap();
            static ref GIT_MV: Regex = Regex::new(r"git\s+mv\s+([^\s]+)\s+([^\s]+)").unwrap();
            static ref GIT_RM: Regex = Regex::new(r"git\s+rm\s+(.+)").unwrap();
            static ref GIT_CHECKOUT: Regex = Regex::new(r"git\s+checkout\s+(.+)").unwrap();
            static ref GIT_MERGE: Regex = Regex::new(r"git\s+merge\s+(.+)").unwrap();
        }

        if let Some(caps) = GIT_ADD.captures(command) {
            let files = Self::parse_file_list(&caps[1]);
            return Some(FileOperation {
                operation_type: FileOperationType::GitAdd,
                file_paths: files,
            });
        }

        if GIT_COMMIT.is_match(command) {
            return Some(FileOperation {
                operation_type: FileOperationType::GitCommit,
                file_paths: vec![],
            });
        }

        if let Some(caps) = GIT_MV.captures(command) {
            return Some(FileOperation {
                operation_type: FileOperationType::GitMove,
                file_paths: vec![caps[1].to_string(), caps[2].to_string()],
            });
        }

        if let Some(caps) = GIT_RM.captures(command) {
            let files = Self::parse_file_list(&caps[1]);
            return Some(FileOperation {
                operation_type: FileOperationType::GitRemove,
                file_paths: files,
            });
        }

        if let Some(caps) = GIT_CHECKOUT.captures(command) {
            return Some(FileOperation {
                operation_type: FileOperationType::GitCheckout,
                file_paths: vec![caps[1].to_string()],
            });
        }

        if let Some(caps) = GIT_MERGE.captures(command) {
            return Some(FileOperation {
                operation_type: FileOperationType::GitMerge,
                file_paths: vec![caps[1].to_string()],
            });
        }

        None
    }

    /// Parse file system commands (excluding git commands)
    fn parse_fs_commands(command: &str) -> Vec<FileOperation> {
        // Skip if this is a git command
        if command.trim().starts_with("git ") {
            return Vec::new();
        }

        lazy_static::lazy_static! {
            static ref MKDIR: Regex = Regex::new(r"^mkdir(?: -p)?\s+(.+)").unwrap();
            static ref TOUCH: Regex = Regex::new(r"^touch\s+(.+)").unwrap();
            static ref CP: Regex = Regex::new(r"^cp(?: -r)?\s+([^\s]+)\s+([^\s]+)").unwrap();
            static ref MV: Regex = Regex::new(r"^mv\s+([^\s]+)\s+([^\s]+)").unwrap();
            static ref RM: Regex = Regex::new(r"^rm(?: -rf?)?\s+(.+)").unwrap();
        }

        let mut operations = Vec::new();

        // mkdir
        if let Some(caps) = MKDIR.captures(command) {
            operations.push(FileOperation {
                operation_type: FileOperationType::Create,
                file_paths: Self::parse_file_list(&caps[1]),
            });
        }

        // touch
        if let Some(caps) = TOUCH.captures(command) {
            operations.push(FileOperation {
                operation_type: FileOperationType::Create,
                file_paths: Self::parse_file_list(&caps[1]),
            });
        }

        // cp
        if let Some(caps) = CP.captures(command) {
            operations.push(FileOperation {
                operation_type: FileOperationType::Copy,
                file_paths: vec![caps[1].to_string(), caps[2].to_string()],
            });
        }

        // mv
        if let Some(caps) = MV.captures(command) {
            operations.push(FileOperation {
                operation_type: FileOperationType::Move,
                file_paths: vec![caps[1].to_string(), caps[2].to_string()],
            });
        }

        // rm
        if let Some(caps) = RM.captures(command) {
            operations.push(FileOperation {
                operation_type: FileOperationType::Delete,
                file_paths: Self::parse_file_list(&caps[1]),
            });
        }

        operations
    }

    /// Parse tooling commands (cargo, npm, etc.)
    fn parse_tooling_commands(command: &str) -> Vec<FileOperation> {
        let mut operations = Vec::new();

        // Cargo operations
        if command.contains("cargo fmt") || command.contains("rustfmt") {
            operations.push(FileOperation {
                operation_type: FileOperationType::Format,
                file_paths: vec!["**/*.rs".to_string()], // Pattern-based
            });
        }

        if command.contains("cargo add") {
            operations.push(FileOperation {
                operation_type: FileOperationType::PackageAdd,
                file_paths: vec!["Cargo.toml".to_string()],
            });
        }

        if command.contains("cargo remove") {
            operations.push(FileOperation {
                operation_type: FileOperationType::PackageRemove,
                file_paths: vec!["Cargo.toml".to_string()],
            });
        }

        if command.contains("cargo build") || command.contains("cargo test") {
            operations.push(FileOperation {
                operation_type: FileOperationType::Build,
                file_paths: vec!["target/".to_string()], // Build artifacts
            });
        }

        // NPM operations
        if command.contains("npm install") || command.contains("yarn add") {
            operations.push(FileOperation {
                operation_type: FileOperationType::PackageAdd,
                file_paths: vec!["package.json".to_string(), "package-lock.json".to_string()],
            });
        }

        if command.contains("npm uninstall") || command.contains("yarn remove") {
            operations.push(FileOperation {
                operation_type: FileOperationType::PackageRemove,
                file_paths: vec!["package.json".to_string(), "package-lock.json".to_string()],
            });
        }

        if command.contains("npm run build") || command.contains("yarn build") {
            operations.push(FileOperation {
                operation_type: FileOperationType::Build,
                file_paths: vec!["dist/".to_string(), "build/".to_string()],
            });
        }

        // Other formatters
        if command.contains("prettier") {
            operations.push(FileOperation {
                operation_type: FileOperationType::Format,
                file_paths: vec!["**/*.{js,ts,jsx,tsx,json,css,md}".to_string()],
            });
        }

        operations
    }

    /// Parse other commands that might affect files
    fn parse_other_commands(command: &str) -> Vec<FileOperation> {
        let mut operations = Vec::new();

        // find commands that might delete files
        if command.contains("find") && command.contains("-delete") {
            operations.push(FileOperation {
                operation_type: FileOperationType::Delete,
                file_paths: vec!["**/*".to_string()], // Pattern-based
            });
        }

        // sed commands that modify files in-place
        if command.contains("sed -i") {
            operations.push(FileOperation {
                operation_type: FileOperationType::Modify,
                file_paths: vec!["**/*".to_string()], // Pattern-based
            });
        }

        operations
    }

    /// Parse a space-separated list of files, handling common patterns
    fn parse_file_list(file_str: &str) -> Vec<String> {
        file_str
            .split_whitespace()
            .filter(|s| !s.starts_with('-')) // Skip flags
            .map(|s| s.trim_matches('"').trim_matches('\'').to_string()) // Remove quotes
            .filter(|s| !s.is_empty())
            .collect()
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

    /// Check if this command has file operations
    pub fn has_file_operations(&self) -> bool {
        !self.file_operations.is_empty()
    }

    /// Get all unique file paths from all operations
    pub fn all_file_paths(&self) -> HashSet<String> {
        self.file_operations
            .iter()
            .flat_map(|op| op.file_paths.iter())
            .cloned()
            .collect()
    }

    /// Get operations by type
    pub fn operations_by_type(&self, op_type: &FileOperationType) -> Vec<&FileOperation> {
        self.file_operations
            .iter()
            .filter(|op| &op.operation_type == op_type)
            .collect()
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
    fn test_bash_parser_basic() {
        let parser = BashParser;
        let tool_use = create_bash_tool_use("ls -la");

        let result = parser.parse(&tool_use);
        assert!(result.is_ok());

        let parsed = result.unwrap();
        assert_eq!(parsed.tool_name, "Bash");

        if let ToolData::Bash(data) = parsed.data {
            assert_eq!(data.command, "ls -la");
            assert_eq!(data.description, Some("Test command".to_string()));
            assert!(data.file_operations.is_empty());
        } else {
            panic!("Expected BashData");
        }
    }

    #[test]
    fn test_git_commands() {
        let parser = BashParser;

        // Test git add
        let tool_use = create_bash_tool_use("git add src/main.rs src/lib.rs");
        let result = parser.parse(&tool_use).unwrap();
        if let ToolData::Bash(data) = result.data {
            assert_eq!(data.file_operations.len(), 1);
            assert_eq!(
                data.file_operations[0].operation_type,
                FileOperationType::GitAdd
            );
            assert_eq!(
                data.file_operations[0].file_paths,
                vec!["src/main.rs", "src/lib.rs"]
            );
        }

        // Test git commit
        let tool_use = create_bash_tool_use("git commit -m 'test'");
        let result = parser.parse(&tool_use).unwrap();
        if let ToolData::Bash(data) = result.data {
            assert_eq!(data.file_operations.len(), 1);
            assert_eq!(
                data.file_operations[0].operation_type,
                FileOperationType::GitCommit
            );
            assert!(data.file_operations[0].file_paths.is_empty());
        }

        // Test git mv
        let tool_use = create_bash_tool_use("git mv old.rs new.rs");
        let result = parser.parse(&tool_use).unwrap();
        if let ToolData::Bash(data) = result.data {
            assert_eq!(data.file_operations.len(), 1);
            assert_eq!(
                data.file_operations[0].operation_type,
                FileOperationType::GitMove
            );
            assert_eq!(data.file_operations[0].file_paths, vec!["old.rs", "new.rs"]);
        }
    }

    #[test]
    fn test_filesystem_commands() {
        let parser = BashParser;

        // Test mkdir
        let tool_use = create_bash_tool_use("mkdir -p src/models");
        let result = parser.parse(&tool_use).unwrap();
        if let ToolData::Bash(data) = result.data {
            assert_eq!(data.file_operations.len(), 1);
            assert_eq!(
                data.file_operations[0].operation_type,
                FileOperationType::Create
            );
            assert_eq!(data.file_operations[0].file_paths, vec!["src/models"]);
        }

        // Test touch
        let tool_use = create_bash_tool_use("touch src/main.rs");
        let result = parser.parse(&tool_use).unwrap();
        if let ToolData::Bash(data) = result.data {
            assert_eq!(data.file_operations.len(), 1);
            assert_eq!(
                data.file_operations[0].operation_type,
                FileOperationType::Create
            );
            assert_eq!(data.file_operations[0].file_paths, vec!["src/main.rs"]);
        }

        // Test cp
        let tool_use = create_bash_tool_use("cp -r src/ backup/");
        let result = parser.parse(&tool_use).unwrap();
        if let ToolData::Bash(data) = result.data {
            assert_eq!(data.file_operations.len(), 1);
            assert_eq!(
                data.file_operations[0].operation_type,
                FileOperationType::Copy
            );
            assert_eq!(data.file_operations[0].file_paths, vec!["src/", "backup/"]);
        }

        // Test mv
        let tool_use = create_bash_tool_use("mv old.txt new.txt");
        let result = parser.parse(&tool_use).unwrap();
        if let ToolData::Bash(data) = result.data {
            assert_eq!(data.file_operations.len(), 1);
            assert_eq!(
                data.file_operations[0].operation_type,
                FileOperationType::Move
            );
            assert_eq!(
                data.file_operations[0].file_paths,
                vec!["old.txt", "new.txt"]
            );
        }

        // Test rm
        let tool_use = create_bash_tool_use("rm -rf temp/");
        let result = parser.parse(&tool_use).unwrap();
        if let ToolData::Bash(data) = result.data {
            assert_eq!(data.file_operations.len(), 1);
            assert_eq!(
                data.file_operations[0].operation_type,
                FileOperationType::Delete
            );
            assert_eq!(data.file_operations[0].file_paths, vec!["temp/"]);
        }
    }

    #[test]
    fn test_tooling_commands() {
        let parser = BashParser;

        // Test cargo fmt
        let tool_use = create_bash_tool_use("cargo fmt");
        let result = parser.parse(&tool_use).unwrap();
        if let ToolData::Bash(data) = result.data {
            assert_eq!(data.file_operations.len(), 1);
            assert_eq!(
                data.file_operations[0].operation_type,
                FileOperationType::Format
            );
            assert_eq!(data.file_operations[0].file_paths, vec!["**/*.rs"]);
        }

        // Test cargo add
        let tool_use = create_bash_tool_use("cargo add serde");
        let result = parser.parse(&tool_use).unwrap();
        if let ToolData::Bash(data) = result.data {
            assert_eq!(data.file_operations.len(), 1);
            assert_eq!(
                data.file_operations[0].operation_type,
                FileOperationType::PackageAdd
            );
            assert_eq!(data.file_operations[0].file_paths, vec!["Cargo.toml"]);
        }

        // Test npm install
        let tool_use = create_bash_tool_use("npm install express");
        let result = parser.parse(&tool_use).unwrap();
        if let ToolData::Bash(data) = result.data {
            assert_eq!(data.file_operations.len(), 1);
            assert_eq!(
                data.file_operations[0].operation_type,
                FileOperationType::PackageAdd
            );
            assert_eq!(
                data.file_operations[0].file_paths,
                vec!["package.json", "package-lock.json"]
            );
        }
    }

    #[test]
    fn test_bash_data_methods() {
        let data = BashData {
            command: "ls -la /tmp".to_string(),
            description: None,
            timeout: None,
            file_operations: vec![],
        };

        assert_eq!(data.base_command(), "ls");
        assert!(!data.is_dangerous());
        assert!(!data.is_mutation());
        assert!(!data.has_file_operations());
    }

    #[test]
    fn test_dangerous_commands() {
        let dangerous = BashData {
            command: "rm -rf /".to_string(),
            description: None,
            timeout: None,
            file_operations: vec![],
        };
        assert!(dangerous.is_dangerous());

        let safe = BashData {
            command: "ls -la".to_string(),
            description: None,
            timeout: None,
            file_operations: vec![],
        };
        assert!(!safe.is_dangerous());
    }

    #[test]
    fn test_mutation_commands() {
        let mutation = BashData {
            command: "rm file.txt".to_string(),
            description: None,
            timeout: None,
            file_operations: vec![],
        };
        assert!(mutation.is_mutation());

        let read_only = BashData {
            command: "ls -la".to_string(),
            description: None,
            timeout: None,
            file_operations: vec![],
        };
        assert!(!read_only.is_mutation());
    }

    #[test]
    fn test_file_operations_aggregation() {
        let data = BashData {
            command: "git add src/main.rs && cargo fmt".to_string(),
            description: None,
            timeout: None,
            file_operations: vec![
                FileOperation {
                    operation_type: FileOperationType::GitAdd,
                    file_paths: vec!["src/main.rs".to_string()],
                },
                FileOperation {
                    operation_type: FileOperationType::Format,
                    file_paths: vec!["**/*.rs".to_string()],
                },
            ],
        };

        assert!(data.has_file_operations());
        assert_eq!(data.all_file_paths().len(), 2);
        assert!(data.all_file_paths().contains("src/main.rs"));
        assert!(data.all_file_paths().contains("**/*.rs"));

        let git_ops = data.operations_by_type(&FileOperationType::GitAdd);
        assert_eq!(git_ops.len(), 1);
    }
}
