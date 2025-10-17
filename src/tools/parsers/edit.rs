use super::{ParsedTool, ToolData, ToolParser};
use crate::models::message::ToolUse;
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};

/// Edit tool parser
pub struct EditParser;

/// Structured data from Edit tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditData {
    /// Path to the file being edited
    pub file_path: String,
    /// Old string being replaced
    pub old_string: Option<String>,
    /// New string to replace with
    pub new_string: Option<String>,
    /// Whether to replace all occurrences
    pub replace_all: Option<bool>,
}

impl ToolParser for EditParser {
    fn parse(&self, tool_use: &ToolUse) -> Result<ParsedTool> {
        let file_path = tool_use
            .input
            .get("file_path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Edit tool missing 'file_path' field"))?
            .to_string();

        let old_string = tool_use
            .input
            .get("old_string")
            .and_then(|v| v.as_str())
            .map(String::from);

        let new_string = tool_use
            .input
            .get("new_string")
            .and_then(|v| v.as_str())
            .map(String::from);

        let replace_all = tool_use.input.get("replace_all").and_then(|v| v.as_bool());

        let data = EditData {
            file_path,
            old_string,
            new_string,
            replace_all,
        };

        Ok(ParsedTool::new(
            tool_use.name.clone(),
            ToolData::Edit(data),
            tool_use.input.clone(),
        ))
    }
}

impl EditData {
    /// Get file extension if any
    pub fn file_extension(&self) -> Option<&str> {
        std::path::Path::new(&self.file_path)
            .extension()
            .and_then(|ext| ext.to_str())
    }

    /// Check if this is a bulk replacement
    pub fn is_bulk_replacement(&self) -> bool {
        self.replace_all.unwrap_or(false)
    }

    /// Estimate the scope of the edit (small, medium, large)
    pub fn edit_scope(&self) -> EditScope {
        let old_len = self.old_string.as_ref().map(|s| s.len()).unwrap_or(0);
        let new_len = self.new_string.as_ref().map(|s| s.len()).unwrap_or(0);
        let total = old_len + new_len;

        if total < 100 {
            EditScope::Small
        } else if total < 500 {
            EditScope::Medium
        } else {
            EditScope::Large
        }
    }

    /// Check if this looks like a refactoring operation (e.g., renaming)
    pub fn is_refactoring(&self) -> bool {
        // Simple heuristic: if old and new strings are similar in length and structure,
        // it might be a refactoring
        if let (Some(old), Some(new)) = (&self.old_string, &self.new_string) {
            let len_diff = (old.len() as i32 - new.len() as i32).abs();
            let is_similar_length = len_diff < 20;
            let has_common_structure =
                old.split_whitespace().count() == new.split_whitespace().count();

            is_similar_length && has_common_structure && self.is_bulk_replacement()
        } else {
            false
        }
    }
}

/// Scope of an edit operation
#[derive(Debug, Clone, PartialEq)]
pub enum EditScope {
    Small,
    Medium,
    Large,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn create_edit_tool_use(file_path: &str, old: &str, new: &str) -> ToolUse {
        ToolUse {
            id: "test_id".to_string(),
            name: "Edit".to_string(),
            input: json!({
                "file_path": file_path,
                "old_string": old,
                "new_string": new
            }),
            raw: json!({}),
        }
    }

    #[test]
    fn test_edit_parser() {
        let parser = EditParser;
        let tool_use = create_edit_tool_use("/path/to/file.rs", "old_name", "new_name");

        let result = parser.parse(&tool_use);
        assert!(result.is_ok());

        let parsed = result.unwrap();
        assert_eq!(parsed.tool_name, "Edit");

        if let ToolData::Edit(data) = parsed.data {
            assert_eq!(data.file_path, "/path/to/file.rs");
            assert_eq!(data.old_string, Some("old_name".to_string()));
            assert_eq!(data.new_string, Some("new_name".to_string()));
        } else {
            panic!("Expected EditData");
        }
    }

    #[test]
    fn test_edit_data_is_bulk_replacement() {
        let bulk = EditData {
            file_path: "/test".to_string(),
            old_string: None,
            new_string: None,
            replace_all: Some(true),
        };
        assert!(bulk.is_bulk_replacement());

        let single = EditData {
            file_path: "/test".to_string(),
            old_string: None,
            new_string: None,
            replace_all: Some(false),
        };
        assert!(!single.is_bulk_replacement());
    }

    #[test]
    fn test_edit_data_edit_scope() {
        let small = EditData {
            file_path: "/test".to_string(),
            old_string: Some("old".to_string()),
            new_string: Some("new".to_string()),
            replace_all: None,
        };
        assert_eq!(small.edit_scope(), EditScope::Small);

        let large = EditData {
            file_path: "/test".to_string(),
            old_string: Some("x".repeat(300)),
            new_string: Some("y".repeat(300)),
            replace_all: None,
        };
        assert_eq!(large.edit_scope(), EditScope::Large);
    }

    #[test]
    fn test_edit_data_is_refactoring() {
        let refactor = EditData {
            file_path: "/test".to_string(),
            old_string: Some("old_function_name".to_string()),
            new_string: Some("new_function_name".to_string()),
            replace_all: Some(true),
        };
        assert!(refactor.is_refactoring());

        let not_refactor = EditData {
            file_path: "/test".to_string(),
            old_string: Some("x".to_string()),
            new_string: Some("completely different content here".to_string()),
            replace_all: Some(true),
        };
        assert!(!not_refactor.is_refactoring());
    }
}
