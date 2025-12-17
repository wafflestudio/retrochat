pub mod parsers;
pub mod types;

pub use types::ToolType;

use crate::models::message::ToolUse;
use anyhow::Result;
use parsers::{
    bash::BashParser, edit::EditParser, read::ReadParser, write::WriteParser, ParsedTool,
    ToolParser,
};

/// Main tool parsing facade
pub struct ToolParsingService;

impl ToolParsingService {
    /// Create a new tool parsing service
    pub fn new() -> Self {
        Self
    }

    /// Parse a tool use into structured data
    pub fn parse_tool(&self, tool_use: &ToolUse) -> Result<ParsedTool> {
        let tool_type = ToolType::from_name(&tool_use.name);

        match tool_type {
            ToolType::Bash => BashParser.parse(tool_use),
            ToolType::Read => ReadParser.parse(tool_use),
            ToolType::Write => WriteParser.parse(tool_use),
            ToolType::Edit => EditParser.parse(tool_use),
            _ => {
                // For unsupported tool types, return unknown parsed tool
                Ok(ParsedTool::new(
                    tool_use.name.clone(),
                    parsers::ToolData::Unknown,
                    tool_use.input.clone(),
                ))
            }
        }
    }

    /// Parse multiple tool uses
    pub fn parse_tools(&self, tool_uses: &[ToolUse]) -> Vec<Result<ParsedTool>> {
        tool_uses
            .iter()
            .map(|tool_use| self.parse_tool(tool_use))
            .collect()
    }

    /// Get tool type from tool use
    pub fn get_tool_type(&self, tool_use: &ToolUse) -> ToolType {
        ToolType::from_name(&tool_use.name)
    }
}

impl Default for ToolParsingService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn create_bash_tool_use() -> ToolUse {
        ToolUse {
            id: "test_id".to_string(),
            name: "Bash".to_string(),
            input: json!({
                "command": "ls -la"
            }),
            raw: json!({}),
        }
    }

    #[test]
    fn test_tool_parsing_service() {
        let service = ToolParsingService::new();
        let tool_use = create_bash_tool_use();

        let result = service.parse_tool(&tool_use);
        assert!(result.is_ok());

        let parsed = result.unwrap();
        assert_eq!(parsed.tool_name, "Bash");
    }

    #[test]
    fn test_get_tool_type() {
        let service = ToolParsingService::new();
        let tool_use = create_bash_tool_use();

        let tool_type = service.get_tool_type(&tool_use);
        assert_eq!(tool_type, ToolType::Bash);
    }

    #[test]
    fn test_parse_multiple_tools() {
        let service = ToolParsingService::new();
        let tools = vec![create_bash_tool_use(), create_bash_tool_use()];

        let results = service.parse_tools(&tools);
        assert_eq!(results.len(), 2);
        assert!(results.iter().all(|r| r.is_ok()));
    }
}
