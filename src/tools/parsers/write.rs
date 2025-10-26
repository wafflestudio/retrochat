use super::{ParsedTool, ToolData, ToolParser};
use crate::models::message::ToolUse;
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};

/// Write tool parser
pub struct WriteParser;

/// Structured data from Write tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WriteData {
    /// Path to the file being written
    pub file_path: String,
    /// Content being written (may be truncated for large content)
    pub content: Option<String>,
    /// Size of content in bytes (if content is truncated)
    pub content_size: Option<usize>,
}

impl ToolParser for WriteParser {
    fn parse(&self, tool_use: &ToolUse) -> Result<ParsedTool> {
        let file_path = tool_use
            .input
            .get("file_path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Write tool missing 'file_path' field"))?
            .to_string();

        let content = tool_use
            .input
            .get("content")
            .and_then(|v| v.as_str())
            .map(String::from);

        let content_size = content.as_ref().map(|c| c.len());

        let data = WriteData {
            file_path,
            content,
            content_size,
        };

        Ok(ParsedTool::new(
            tool_use.name.clone(),
            ToolData::Write(data),
            tool_use.input.clone(),
        ))
    }
}

impl WriteData {
    /// Get file extension if any
    pub fn file_extension(&self) -> Option<&str> {
        std::path::Path::new(&self.file_path)
            .extension()
            .and_then(|ext| ext.to_str())
    }

    /// Check if writing a configuration file
    pub fn is_config_file(&self) -> bool {
        let config_extensions = ["json", "yaml", "yml", "toml", "ini", "conf", "config"];
        self.file_extension()
            .map(|ext| config_extensions.contains(&ext))
            .unwrap_or(false)
    }

    /// Check if writing a code file
    pub fn is_code_file(&self) -> bool {
        let code_extensions = [
            "rs", "py", "js", "ts", "tsx", "jsx", "go", "java", "c", "cpp", "h", "hpp",
        ];
        self.file_extension()
            .map(|ext| code_extensions.contains(&ext))
            .unwrap_or(false)
    }

    /// Check if this is a large write operation (>10KB)
    pub fn is_large_write(&self) -> bool {
        self.content_size.map(|size| size > 10240).unwrap_or(false)
    }

    /// Count lines in content
    pub fn lines_after(&self) -> Option<i32> {
        self.content.as_ref().map(|s| {
            if s.is_empty() {
                return 0;
            }
            let newline_count = s.chars().filter(|&c| c == '\n').count();
            if s.ends_with('\n') {
                newline_count as i32
            } else {
                (newline_count + 1) as i32
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn create_write_tool_use(file_path: &str, content: &str) -> ToolUse {
        ToolUse {
            id: "test_id".to_string(),
            name: "Write".to_string(),
            input: json!({
                "file_path": file_path,
                "content": content
            }),
            raw: json!({}),
        }
    }

    #[test]
    fn test_write_parser() {
        let parser = WriteParser;
        let tool_use = create_write_tool_use("/path/to/file.rs", "fn main() {}");

        let result = parser.parse(&tool_use);
        assert!(result.is_ok());

        let parsed = result.unwrap();
        assert_eq!(parsed.tool_name, "Write");

        if let ToolData::Write(data) = parsed.data {
            assert_eq!(data.file_path, "/path/to/file.rs");
            assert_eq!(data.content, Some("fn main() {}".to_string()));
            assert_eq!(data.content_size, Some("fn main() {}".len()));
        } else {
            panic!("Expected WriteData");
        }
    }

    #[test]
    fn test_write_data_file_extension() {
        let data = WriteData {
            file_path: "/path/to/file.rs".to_string(),
            content: None,
            content_size: None,
        };
        assert_eq!(data.file_extension(), Some("rs"));
    }

    #[test]
    fn test_write_data_is_config_file() {
        let config = WriteData {
            file_path: "/path/config.json".to_string(),
            content: None,
            content_size: None,
        };
        assert!(config.is_config_file());

        let not_config = WriteData {
            file_path: "/path/file.rs".to_string(),
            content: None,
            content_size: None,
        };
        assert!(!not_config.is_config_file());
    }

    #[test]
    fn test_write_data_is_large_write() {
        let small = WriteData {
            file_path: "/test".to_string(),
            content: Some("small".to_string()),
            content_size: Some(5),
        };
        assert!(!small.is_large_write());

        let large = WriteData {
            file_path: "/test".to_string(),
            content: None,
            content_size: Some(20000),
        };
        assert!(large.is_large_write());
    }
}
