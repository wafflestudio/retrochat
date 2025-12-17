use serde::{Deserialize, Serialize};

/// Unified tool types across all providers
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ToolType {
    /// Bash/shell command execution
    Bash,
    /// File reading operations
    Read,
    /// File writing operations
    Write,
    /// File editing operations
    Edit,
    /// Directory/file globbing
    Glob,
    /// Text/code searching (grep)
    Grep,
    /// Web fetching operations
    WebFetch,
    /// Web search operations
    WebSearch,
    /// Task/agent launching
    Task,
    /// Notebook editing
    NotebookEdit,
    /// Other/unknown tool type
    Other(String),
}

impl ToolType {
    /// Detect tool type from tool name
    pub fn from_name(name: &str) -> Self {
        match name {
            "Bash" | "bash" | "shell" => ToolType::Bash,
            "Read" | "read" | "read_file" => ToolType::Read,
            "Write" | "write" | "write_file" => ToolType::Write,
            "Edit" | "edit" | "edit_file" => ToolType::Edit,
            "Glob" | "glob" | "find" => ToolType::Glob,
            "Grep" | "grep" | "search" => ToolType::Grep,
            "WebFetch" | "web_fetch" | "fetch" => ToolType::WebFetch,
            "WebSearch" | "web_search" => ToolType::WebSearch,
            "Task" | "task" | "agent" => ToolType::Task,
            "NotebookEdit" | "notebook_edit" => ToolType::NotebookEdit,
            other => ToolType::Other(other.to_string()),
        }
    }

    /// Get canonical name for the tool type
    pub fn canonical_name(&self) -> &str {
        match self {
            ToolType::Bash => "Bash",
            ToolType::Read => "Read",
            ToolType::Write => "Write",
            ToolType::Edit => "Edit",
            ToolType::Glob => "Glob",
            ToolType::Grep => "Grep",
            ToolType::WebFetch => "WebFetch",
            ToolType::WebSearch => "WebSearch",
            ToolType::Task => "Task",
            ToolType::NotebookEdit => "NotebookEdit",
            ToolType::Other(name) => name,
        }
    }

    /// Check if this is a file operation tool
    pub fn is_file_operation(&self) -> bool {
        matches!(
            self,
            ToolType::Read | ToolType::Write | ToolType::Edit | ToolType::Glob
        )
    }

    /// Check if this is a network operation tool
    pub fn is_network_operation(&self) -> bool {
        matches!(self, ToolType::WebFetch | ToolType::WebSearch)
    }

    /// Check if this is a code operation tool
    pub fn is_code_operation(&self) -> bool {
        matches!(self, ToolType::Grep | ToolType::NotebookEdit)
    }
}

impl std::fmt::Display for ToolType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.canonical_name())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_type_from_name() {
        assert_eq!(ToolType::from_name("Bash"), ToolType::Bash);
        assert_eq!(ToolType::from_name("bash"), ToolType::Bash);
        assert_eq!(ToolType::from_name("Read"), ToolType::Read);
        assert_eq!(ToolType::from_name("read_file"), ToolType::Read);
        assert_eq!(
            ToolType::from_name("custom_tool"),
            ToolType::Other("custom_tool".to_string())
        );
    }

    #[test]
    fn test_tool_type_canonical_name() {
        assert_eq!(ToolType::Bash.canonical_name(), "Bash");
        assert_eq!(ToolType::Read.canonical_name(), "Read");
        assert_eq!(
            ToolType::Other("custom".to_string()).canonical_name(),
            "custom"
        );
    }

    #[test]
    fn test_tool_type_categories() {
        assert!(ToolType::Read.is_file_operation());
        assert!(ToolType::Write.is_file_operation());
        assert!(!ToolType::Bash.is_file_operation());

        assert!(ToolType::WebFetch.is_network_operation());
        assert!(ToolType::WebSearch.is_network_operation());
        assert!(!ToolType::Bash.is_network_operation());

        assert!(ToolType::Grep.is_code_operation());
        assert!(!ToolType::Bash.is_code_operation());
    }

    #[test]
    fn test_tool_type_display() {
        assert_eq!(ToolType::Bash.to_string(), "Bash");
        assert_eq!(ToolType::Other("test".to_string()).to_string(), "test");
    }
}
