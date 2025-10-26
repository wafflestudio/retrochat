use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use super::message::{ToolResult, ToolUse};

/// File-related metadata for tool operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileMetadata {
    pub file_path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_extension: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_code_file: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_config_file: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lines_before: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lines_after: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lines_added: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lines_removed: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_size: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_bulk_edit: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_refactoring: Option<bool>,
}

impl FileMetadata {
    pub fn new(file_path: String) -> Self {
        let file_extension = std::path::Path::new(&file_path)
            .extension()
            .and_then(|ext| ext.to_str())
            .map(String::from);

        Self {
            file_path,
            file_extension,
            is_code_file: None,
            is_config_file: None,
            lines_before: None,
            lines_after: None,
            lines_added: None,
            lines_removed: None,
            content_size: None,
            is_bulk_edit: None,
            is_refactoring: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolOperation {
    pub id: Uuid,
    pub tool_use_id: String,
    pub tool_name: String,
    pub timestamp: DateTime<Utc>,

    // File-related metadata (None for non-file tools)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_metadata: Option<FileMetadata>,

    // Generic fields for all tools
    pub success: Option<bool>,
    pub result_summary: Option<String>,
    pub raw_input: Option<Value>,
    pub raw_result: Option<Value>,

    pub created_at: DateTime<Utc>,
}

impl ToolOperation {
    pub fn new(tool_use_id: String, tool_name: String, timestamp: DateTime<Utc>) -> Self {
        Self {
            id: Uuid::new_v4(),
            tool_use_id,
            tool_name,
            timestamp,
            file_metadata: None,
            success: None,
            result_summary: None,
            raw_input: None,
            raw_result: None,
            created_at: Utc::now(),
        }
    }

    /// Check if this operation involves file manipulation
    pub fn is_file_operation(&self) -> bool {
        self.file_metadata.is_some()
    }

    /// Check if this operation modified a code file
    pub fn is_code_modification(&self) -> bool {
        self.file_metadata
            .as_ref()
            .and_then(|meta| meta.is_code_file)
            .unwrap_or(false)
            && (self.tool_name == "Write" || self.tool_name == "Edit")
    }

    /// Get total line changes (added + removed)
    pub fn total_line_changes(&self) -> i32 {
        if let Some(meta) = &self.file_metadata {
            let added = meta.lines_added.unwrap_or(0);
            let removed = meta.lines_removed.unwrap_or(0);
            added + removed
        } else {
            0
        }
    }

    /// Calculate net line change (added - removed)
    pub fn net_line_change(&self) -> i32 {
        if let Some(meta) = &self.file_metadata {
            let added = meta.lines_added.unwrap_or(0);
            let removed = meta.lines_removed.unwrap_or(0);
            added - removed
        } else {
            0
        }
    }

    /// Builder method: set file path and create metadata
    pub fn with_file_path(mut self, file_path: String) -> Self {
        self.file_metadata = Some(FileMetadata::new(file_path));
        self
    }

    /// Builder method: set file type flags
    pub fn with_file_type(mut self, is_code: bool, is_config: bool) -> Self {
        if let Some(meta) = &mut self.file_metadata {
            meta.is_code_file = Some(is_code);
            meta.is_config_file = Some(is_config);
        }
        self
    }

    /// Builder method: set line metrics
    pub fn with_line_metrics(
        mut self,
        lines_before: Option<i32>,
        lines_after: Option<i32>,
    ) -> Self {
        if let Some(meta) = &mut self.file_metadata {
            meta.lines_before = lines_before;
            meta.lines_after = lines_after;

            // Calculate added/removed based on before/after
            if let (Some(before), Some(after)) = (lines_before, lines_after) {
                if after > before {
                    meta.lines_added = Some(after - before);
                    meta.lines_removed = Some(0);
                } else if before > after {
                    meta.lines_added = Some(0);
                    meta.lines_removed = Some(before - after);
                } else {
                    meta.lines_added = Some(0);
                    meta.lines_removed = Some(0);
                }
            }
        }
        self
    }

    /// Builder method: set content size
    pub fn with_content_size(mut self, size: i32) -> Self {
        if let Some(meta) = &mut self.file_metadata {
            meta.content_size = Some(size);
        }
        self
    }

    /// Builder method: set edit-specific flags
    pub fn with_edit_flags(mut self, is_bulk: bool, is_refactoring: bool) -> Self {
        if let Some(meta) = &mut self.file_metadata {
            meta.is_bulk_edit = Some(is_bulk);
            meta.is_refactoring = Some(is_refactoring);
        }
        self
    }

    /// Builder method: set success status
    pub fn with_success(mut self, success: bool) -> Self {
        self.success = Some(success);
        self
    }

    /// Builder method: set result summary (truncated)
    pub fn with_result_summary(mut self, summary: String) -> Self {
        // Truncate to 500 chars for summary (UTF-8 safe)
        let truncated = if summary.len() > 500 {
            // Find the last valid char boundary before byte 497
            let mut end_idx = 497.min(summary.len());
            while end_idx > 0 && !summary.is_char_boundary(end_idx) {
                end_idx -= 1;
            }
            format!("{}...", &summary[..end_idx])
        } else {
            summary
        };
        self.result_summary = Some(truncated);
        self
    }

    /// Builder method: set raw input
    pub fn with_raw_input(mut self, input: Value) -> Self {
        self.raw_input = Some(input);
        self
    }

    /// Builder method: set raw result
    pub fn with_raw_result(mut self, result: Value) -> Self {
        self.raw_result = Some(result);
        self
    }

    /// Count lines in a string by counting newline characters
    pub fn count_lines(text: &str) -> i32 {
        if text.is_empty() {
            return 0;
        }
        // Count newlines + 1 (for the last line if it doesn't end with newline)
        let newline_count = text.chars().filter(|&c| c == '\n').count();
        if text.ends_with('\n') {
            newline_count as i32
        } else {
            (newline_count + 1) as i32
        }
    }

    /// Create ToolOperation from ToolUse and optionally ToolResult
    pub fn from_tool_use(
        tool_use: &ToolUse,
        tool_result: Option<&ToolResult>,
        timestamp: DateTime<Utc>,
    ) -> Self {
        let mut operation =
            ToolOperation::new(tool_use.id.clone(), tool_use.name.clone(), timestamp);

        operation = operation.with_raw_input(tool_use.input.clone());

        if let Some(result) = tool_result {
            operation = operation
                .with_success(!result.is_error)
                .with_result_summary(result.content.clone());

            if let Some(details) = &result.details {
                operation = operation.with_raw_result(details.clone());
            }
        }

        operation
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_count_lines_empty() {
        assert_eq!(ToolOperation::count_lines(""), 0);
    }

    #[test]
    fn test_count_lines_single_line() {
        assert_eq!(ToolOperation::count_lines("hello"), 1);
    }

    #[test]
    fn test_count_lines_with_newline() {
        assert_eq!(ToolOperation::count_lines("hello\n"), 1);
    }

    #[test]
    fn test_count_lines_multiple() {
        assert_eq!(ToolOperation::count_lines("hello\nworld"), 2);
        assert_eq!(ToolOperation::count_lines("hello\nworld\n"), 2);
        assert_eq!(ToolOperation::count_lines("a\nb\nc"), 3);
        assert_eq!(ToolOperation::count_lines("a\nb\nc\n"), 3);
    }

    #[test]
    fn test_new_tool_operation() {
        let tool_use_id = "test_tool_use".to_string();
        let tool_name = "Write".to_string();
        let timestamp = Utc::now();

        let op = ToolOperation::new(tool_use_id.clone(), tool_name.clone(), timestamp);

        assert_eq!(op.tool_use_id, tool_use_id);
        assert_eq!(op.tool_name, tool_name);
        assert!(!op.is_file_operation());
    }

    #[test]
    fn test_with_file_path() {
        let op = ToolOperation::new("test".to_string(), "Write".to_string(), Utc::now())
            .with_file_path("/path/to/file.rs".to_string());

        assert!(op.is_file_operation());
        let meta = op.file_metadata.as_ref().unwrap();
        assert_eq!(meta.file_path, "/path/to/file.rs".to_string());
        assert_eq!(meta.file_extension, Some("rs".to_string()));
    }

    #[test]
    fn test_with_line_metrics() {
        let op = ToolOperation::new("test".to_string(), "Edit".to_string(), Utc::now())
            .with_file_path("/path/to/file.rs".to_string())
            .with_line_metrics(Some(10), Some(15));

        let meta = op.file_metadata.as_ref().unwrap();
        assert_eq!(meta.lines_before, Some(10));
        assert_eq!(meta.lines_after, Some(15));
        assert_eq!(meta.lines_added, Some(5));
        assert_eq!(meta.lines_removed, Some(0));
        assert_eq!(op.total_line_changes(), 5);
        assert_eq!(op.net_line_change(), 5);
    }

    #[test]
    fn test_with_line_metrics_removal() {
        let op = ToolOperation::new("test".to_string(), "Edit".to_string(), Utc::now())
            .with_file_path("/path/to/file.rs".to_string())
            .with_line_metrics(Some(20), Some(15));

        let meta = op.file_metadata.as_ref().unwrap();
        assert_eq!(meta.lines_added, Some(0));
        assert_eq!(meta.lines_removed, Some(5));
        assert_eq!(op.total_line_changes(), 5);
        assert_eq!(op.net_line_change(), -5);
    }

    #[test]
    fn test_is_code_modification() {
        let op = ToolOperation::new("test".to_string(), "Write".to_string(), Utc::now())
            .with_file_path("/path/to/file.rs".to_string())
            .with_file_type(true, false);

        assert!(op.is_code_modification());

        let read_op = ToolOperation::new("test".to_string(), "Read".to_string(), Utc::now())
            .with_file_path("/path/to/file.rs".to_string())
            .with_file_type(true, false);

        assert!(!read_op.is_code_modification());
    }

    #[test]
    fn test_result_summary_truncation() {
        let long_text = "a".repeat(600);
        let op = ToolOperation::new("test".to_string(), "Bash".to_string(), Utc::now())
            .with_result_summary(long_text);

        assert!(op.result_summary.is_some());
        let summary = op.result_summary.unwrap();
        assert_eq!(summary.len(), 500);
        assert!(summary.ends_with("..."));
    }

    #[test]
    fn test_result_summary_truncation_with_utf8() {
        // Test with Korean text (3 bytes per char)
        let korean_text = "안녕하세요".repeat(150); // ~2250 bytes
        let op = ToolOperation::new("test".to_string(), "Bash".to_string(), Utc::now())
            .with_result_summary(korean_text);

        assert!(op.result_summary.is_some());
        let summary = op.result_summary.unwrap();
        // Should be truncated but remain UTF-8 valid
        assert!(summary.len() <= 500);
        assert!(summary.ends_with("..."));
        // Should not panic when converting to string (validates UTF-8)
        assert!(summary.chars().count() > 0);
    }

    #[test]
    fn test_result_summary_truncation_with_mixed_chars() {
        // Mix of ASCII, Korean, and special chars
        let mixed = "Hello 안녕 → │ ".repeat(50); // Mixed byte sizes
        let op = ToolOperation::new("test".to_string(), "Bash".to_string(), Utc::now())
            .with_result_summary(mixed);

        assert!(op.result_summary.is_some());
        let summary = op.result_summary.unwrap();
        // Should be truncated safely at char boundary
        assert!(summary.len() <= 500);
        assert!(summary.ends_with("..."));
        // Validate UTF-8 integrity
        assert!(std::str::from_utf8(summary.as_bytes()).is_ok());
    }
}
