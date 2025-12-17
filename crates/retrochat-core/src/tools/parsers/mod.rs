pub mod bash;
pub mod edit;
pub mod read;
pub mod write;

use crate::models::message::ToolUse;
use anyhow::Result;
use serde_json::Value;

/// Common trait for tool-specific parsers
pub trait ToolParser {
    /// Parse tool input and extract structured data
    fn parse(&self, tool_use: &ToolUse) -> Result<ParsedTool>;
}

/// Parsed tool data with structured information
#[derive(Debug, Clone)]
pub struct ParsedTool {
    /// Original tool name
    pub tool_name: String,
    /// Structured tool-specific data
    pub data: ToolData,
    /// Original raw input
    pub raw_input: Value,
}

/// Tool-specific structured data
#[derive(Debug, Clone)]
pub enum ToolData {
    Bash(bash::BashData),
    Read(read::ReadData),
    Write(write::WriteData),
    Edit(edit::EditData),
    Unknown,
}

impl ParsedTool {
    /// Create a new parsed tool
    pub fn new(tool_name: String, data: ToolData, raw_input: Value) -> Self {
        Self {
            tool_name,
            data,
            raw_input,
        }
    }
}
