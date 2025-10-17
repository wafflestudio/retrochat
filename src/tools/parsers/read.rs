use super::{ParsedTool, ToolData, ToolParser};
use crate::models::message::ToolUse;
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};

/// Read tool parser
pub struct ReadParser;

/// Structured data from Read tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadData {
    /// Path to the file being read
    pub file_path: String,
    /// Optional line offset for partial reads
    pub offset: Option<u64>,
    /// Optional line limit for partial reads
    pub limit: Option<u64>,
}

impl ToolParser for ReadParser {
    fn parse(&self, tool_use: &ToolUse) -> Result<ParsedTool> {
        let file_path = tool_use
            .input
            .get("file_path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Read tool missing 'file_path' field"))?
            .to_string();

        let offset = tool_use.input.get("offset").and_then(|v| v.as_u64());
        let limit = tool_use.input.get("limit").and_then(|v| v.as_u64());

        let data = ReadData {
            file_path,
            offset,
            limit,
        };

        Ok(ParsedTool::new(
            tool_use.name.clone(),
            ToolData::Read(data),
            tool_use.input.clone(),
        ))
    }
}

impl ReadData {
    /// Check if this is a partial read (with offset or limit)
    pub fn is_partial_read(&self) -> bool {
        self.offset.is_some() || self.limit.is_some()
    }

    /// Get file extension if any
    pub fn file_extension(&self) -> Option<&str> {
        std::path::Path::new(&self.file_path)
            .extension()
            .and_then(|ext| ext.to_str())
    }

    /// Check if reading a configuration file
    pub fn is_config_file(&self) -> bool {
        let config_extensions = ["json", "yaml", "yml", "toml", "ini", "conf", "config"];
        self.file_extension()
            .map(|ext| config_extensions.contains(&ext))
            .unwrap_or(false)
    }

    /// Check if reading a code file
    pub fn is_code_file(&self) -> bool {
        let code_extensions = [
            "rs", "py", "js", "ts", "tsx", "jsx", "go", "java", "c", "cpp", "h", "hpp",
        ];
        self.file_extension()
            .map(|ext| code_extensions.contains(&ext))
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn create_read_tool_use(file_path: &str) -> ToolUse {
        ToolUse {
            id: "test_id".to_string(),
            name: "Read".to_string(),
            input: json!({
                "file_path": file_path
            }),
            raw: json!({}),
        }
    }

    #[test]
    fn test_read_parser() {
        let parser = ReadParser;
        let tool_use = create_read_tool_use("/path/to/file.rs");

        let result = parser.parse(&tool_use);
        assert!(result.is_ok());

        let parsed = result.unwrap();
        assert_eq!(parsed.tool_name, "Read");

        if let ToolData::Read(data) = parsed.data {
            assert_eq!(data.file_path, "/path/to/file.rs");
            assert_eq!(data.offset, None);
            assert_eq!(data.limit, None);
        } else {
            panic!("Expected ReadData");
        }
    }

    #[test]
    fn test_read_data_partial_read() {
        let full = ReadData {
            file_path: "/test".to_string(),
            offset: None,
            limit: None,
        };
        assert!(!full.is_partial_read());

        let partial = ReadData {
            file_path: "/test".to_string(),
            offset: Some(10),
            limit: Some(50),
        };
        assert!(partial.is_partial_read());
    }

    #[test]
    fn test_read_data_file_extension() {
        let data = ReadData {
            file_path: "/path/to/file.rs".to_string(),
            offset: None,
            limit: None,
        };
        assert_eq!(data.file_extension(), Some("rs"));
    }

    #[test]
    fn test_read_data_is_config_file() {
        let config = ReadData {
            file_path: "/path/config.json".to_string(),
            offset: None,
            limit: None,
        };
        assert!(config.is_config_file());

        let not_config = ReadData {
            file_path: "/path/file.rs".to_string(),
            offset: None,
            limit: None,
        };
        assert!(!not_config.is_config_file());
    }

    #[test]
    fn test_read_data_is_code_file() {
        let code = ReadData {
            file_path: "/path/file.rs".to_string(),
            offset: None,
            limit: None,
        };
        assert!(code.is_code_file());

        let not_code = ReadData {
            file_path: "/path/file.txt".to_string(),
            offset: None,
            limit: None,
        };
        assert!(!not_code.is_code_file());
    }
}
