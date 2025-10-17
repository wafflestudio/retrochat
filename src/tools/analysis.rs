use crate::models::message::{Message, ToolUse};
use crate::tools::types::ToolType;
use std::collections::HashMap;

/// Statistics about tool usage
#[derive(Debug, Clone)]
pub struct ToolUsageStats {
    /// Total number of tool uses
    pub total_tools: usize,
    /// Count by tool type
    pub by_type: HashMap<String, usize>,
    /// Count of file operations
    pub file_operations: usize,
    /// Count of network operations
    pub network_operations: usize,
    /// Count of code operations
    pub code_operations: usize,
}

impl ToolUsageStats {
    /// Create new empty stats
    pub fn new() -> Self {
        Self {
            total_tools: 0,
            by_type: HashMap::new(),
            file_operations: 0,
            network_operations: 0,
            code_operations: 0,
        }
    }

    /// Add a tool use to the statistics
    pub fn add_tool(&mut self, tool_use: &ToolUse) {
        self.total_tools += 1;

        let tool_type = ToolType::from_name(&tool_use.name);
        *self.by_type.entry(tool_type.to_string()).or_insert(0) += 1;

        if tool_type.is_file_operation() {
            self.file_operations += 1;
        }
        if tool_type.is_network_operation() {
            self.network_operations += 1;
        }
        if tool_type.is_code_operation() {
            self.code_operations += 1;
        }
    }

    /// Get the most frequently used tool type
    pub fn most_used_tool(&self) -> Option<(&String, &usize)> {
        self.by_type.iter().max_by_key(|(_, count)| *count)
    }

    /// Get percentage of file operations
    pub fn file_operations_percentage(&self) -> f64 {
        if self.total_tools == 0 {
            0.0
        } else {
            (self.file_operations as f64 / self.total_tools as f64) * 100.0
        }
    }
}

impl Default for ToolUsageStats {
    fn default() -> Self {
        Self::new()
    }
}

/// Analyze tool usage from messages
pub fn analyze_tool_usage(messages: &[Message]) -> ToolUsageStats {
    let mut stats = ToolUsageStats::new();

    for message in messages {
        if let Some(tool_uses) = &message.tool_uses {
            for tool_use in tool_uses {
                stats.add_tool(tool_use);
            }
        }
    }

    stats
}

/// Find messages with specific tool type
pub fn find_messages_with_tool(messages: &[Message], tool_type: ToolType) -> Vec<&Message> {
    messages
        .iter()
        .filter(|msg| {
            msg.tool_uses
                .as_ref()
                .map(|uses| {
                    uses.iter().any(|use_| {
                        ToolType::from_name(&use_.name).canonical_name()
                            == tool_type.canonical_name()
                    })
                })
                .unwrap_or(false)
        })
        .collect()
}

/// Extract all file paths from tool uses in messages
pub fn extract_file_paths(messages: &[Message]) -> Vec<String> {
    let mut paths = Vec::new();

    for message in messages {
        if let Some(tool_uses) = &message.tool_uses {
            for tool_use in tool_uses {
                let tool_type = ToolType::from_name(&tool_use.name);
                if tool_type.is_file_operation() {
                    if let Some(path) = tool_use.input.get("file_path").and_then(|v| v.as_str()) {
                        paths.push(path.to_string());
                    }
                }
            }
        }
    }

    paths
}

/// Count tool uses by vendor type
pub fn count_by_vendor(messages: &[Message]) -> HashMap<String, usize> {
    let mut counts = HashMap::new();

    for message in messages {
        if let Some(tool_uses) = &message.tool_uses {
            for tool_use in tool_uses {
                *counts.entry(tool_use.vendor_type.clone()).or_insert(0) += 1;
            }
        }
    }

    counts
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::message::MessageRole;
    use chrono::Utc;
    use serde_json::json;
    use uuid::Uuid;

    fn create_test_message_with_tools(tools: Vec<ToolUse>) -> Message {
        Message::new(
            Uuid::new_v4(),
            MessageRole::Assistant,
            "test".to_string(),
            Utc::now(),
            1,
        )
        .with_tool_uses(tools)
    }

    fn create_tool_use(name: &str, vendor_type: &str) -> ToolUse {
        ToolUse {
            id: Uuid::new_v4().to_string(),
            name: name.to_string(),
            input: json!({}),
            vendor_type: vendor_type.to_string(),
            raw: json!({}),
        }
    }

    #[test]
    fn test_tool_usage_stats() {
        let mut stats = ToolUsageStats::new();

        let bash_tool = create_tool_use("Bash", "tool_use");
        let read_tool = create_tool_use("Read", "tool_use");

        stats.add_tool(&bash_tool);
        stats.add_tool(&read_tool);
        stats.add_tool(&read_tool);

        assert_eq!(stats.total_tools, 3);
        assert_eq!(*stats.by_type.get("Bash").unwrap(), 1);
        assert_eq!(*stats.by_type.get("Read").unwrap(), 2);
        assert_eq!(stats.file_operations, 2); // Read is a file operation
    }

    #[test]
    fn test_analyze_tool_usage() {
        let tools = vec![
            create_tool_use("Bash", "tool_use"),
            create_tool_use("Read", "tool_use"),
        ];
        let messages = vec![create_test_message_with_tools(tools)];

        let stats = analyze_tool_usage(&messages);

        assert_eq!(stats.total_tools, 2);
        assert!(stats.by_type.contains_key("Bash"));
        assert!(stats.by_type.contains_key("Read"));
    }

    #[test]
    fn test_find_messages_with_tool() {
        let bash_tools = vec![create_tool_use("Bash", "tool_use")];
        let read_tools = vec![create_tool_use("Read", "tool_use")];

        let messages = vec![
            create_test_message_with_tools(bash_tools),
            create_test_message_with_tools(read_tools),
        ];

        let bash_messages = find_messages_with_tool(&messages, ToolType::Bash);
        assert_eq!(bash_messages.len(), 1);

        let read_messages = find_messages_with_tool(&messages, ToolType::Read);
        assert_eq!(read_messages.len(), 1);
    }

    #[test]
    fn test_extract_file_paths() {
        let tools = vec![ToolUse {
            id: "test".to_string(),
            name: "Read".to_string(),
            input: json!({"file_path": "/path/to/file.rs"}),
            vendor_type: "tool_use".to_string(),
            raw: json!({}),
        }];

        let messages = vec![create_test_message_with_tools(tools)];

        let paths = extract_file_paths(&messages);
        assert_eq!(paths.len(), 1);
        assert_eq!(paths[0], "/path/to/file.rs");
    }

    #[test]
    fn test_count_by_vendor() {
        let tools = vec![
            create_tool_use("Bash", "tool_use"),
            create_tool_use("Read", "tool-call"),
        ];

        let messages = vec![create_test_message_with_tools(tools)];

        let counts = count_by_vendor(&messages);
        assert_eq!(*counts.get("tool_use").unwrap(), 1);
        assert_eq!(*counts.get("tool-call").unwrap(), 1);
    }

    #[test]
    fn test_most_used_tool() {
        let mut stats = ToolUsageStats::new();

        stats.add_tool(&create_tool_use("Bash", "tool_use"));
        stats.add_tool(&create_tool_use("Read", "tool_use"));
        stats.add_tool(&create_tool_use("Read", "tool_use"));

        let (tool, count) = stats.most_used_tool().unwrap();
        assert_eq!(tool, "Read");
        assert_eq!(*count, 2);
    }
}
