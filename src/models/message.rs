use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MessageRole {
    User,
    Assistant,
    System,
}

impl std::fmt::Display for MessageRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MessageRole::User => write!(f, "User"),
            MessageRole::Assistant => write!(f, "Assistant"),
            MessageRole::System => write!(f, "System"),
        }
    }
}

impl std::str::FromStr for MessageRole {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "User" => Ok(MessageRole::User),
            "Assistant" => Ok(MessageRole::Assistant),
            "System" => Ok(MessageRole::System),
            _ => Err(format!("Unknown message role: {s}")),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub function: String,
    pub arguments: Value,
    pub result: Option<Value>,
}

/// Unified tool request structure (works across all vendors)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolUse {
    /// Tool execution ID (from vendor or generated)
    pub id: String,
    /// Normalized tool name: "Bash", "Read", "Write", "Edit", etc.
    pub name: String,
    /// Tool-specific input parameters
    pub input: Value,
    /// Complete original JSON for future reference
    pub raw: Value,
}

/// Unified tool response structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    /// Links back to ToolUse.id
    pub tool_use_id: String,
    /// Primary result content
    pub content: String,
    /// Whether this result represents an error
    pub is_error: bool,
    /// Structured result data (stdout, patches, etc.)
    pub details: Option<Value>,
    /// Complete original JSON
    pub raw: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: Uuid,
    pub session_id: Uuid,
    pub role: MessageRole,
    pub content: String,
    pub timestamp: DateTime<Utc>,
    pub token_count: Option<u32>,
    pub tool_calls: Option<Vec<ToolCall>>,
    pub metadata: Option<Value>,
    pub sequence_number: u32,
    /// Unified tool requests (normalized across vendors)
    pub tool_uses: Option<Vec<ToolUse>>,
    /// Unified tool responses (normalized across vendors)
    pub tool_results: Option<Vec<ToolResult>>,
}

impl Message {
    pub fn new(
        session_id: Uuid,
        role: MessageRole,
        content: String,
        timestamp: DateTime<Utc>,
        sequence_number: u32,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            session_id,
            role,
            content,
            timestamp,
            token_count: None,
            tool_calls: None,
            metadata: None,
            sequence_number,
            tool_uses: None,
            tool_results: None,
        }
    }

    pub fn with_token_count(mut self, token_count: u32) -> Self {
        self.token_count = Some(token_count);
        self
    }

    pub fn with_tool_calls(mut self, tool_calls: Vec<ToolCall>) -> Self {
        self.tool_calls = Some(tool_calls);
        self
    }

    pub fn with_metadata(mut self, metadata: Value) -> Self {
        self.metadata = Some(metadata);
        self
    }

    pub fn with_tool_uses(mut self, tool_uses: Vec<ToolUse>) -> Self {
        self.tool_uses = Some(tool_uses);
        self
    }

    pub fn with_tool_results(mut self, tool_results: Vec<ToolResult>) -> Self {
        self.tool_results = Some(tool_results);
        self
    }

    pub fn is_valid(&self) -> bool {
        !self.content.is_empty()
    }

    pub fn has_tool_calls(&self) -> bool {
        self.tool_calls
            .as_ref()
            .is_some_and(|calls| !calls.is_empty())
    }

    pub fn content_length(&self) -> usize {
        self.content.len()
    }

    pub fn word_count(&self) -> usize {
        self.content.split_whitespace().count()
    }

    pub fn is_user_message(&self) -> bool {
        matches!(self.role, MessageRole::User)
    }

    pub fn is_assistant_message(&self) -> bool {
        matches!(self.role, MessageRole::Assistant)
    }

    pub fn is_system_message(&self) -> bool {
        matches!(self.role, MessageRole::System)
    }
}

impl PartialOrd for Message {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Message {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.sequence_number.cmp(&other.sequence_number)
    }
}

impl PartialEq for Message {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for Message {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_message() {
        let session_id = Uuid::new_v4();
        let content = "Hello, world!".to_string();
        let timestamp = Utc::now();
        let sequence_number = 1;

        let message = Message::new(
            session_id,
            MessageRole::User,
            content.clone(),
            timestamp,
            sequence_number,
        );

        assert_eq!(message.session_id, session_id);
        assert_eq!(message.role, MessageRole::User);
        assert_eq!(message.content, content);
        assert_eq!(message.timestamp, timestamp);
        assert_eq!(message.sequence_number, sequence_number);
        assert!(message.is_valid());
        assert!(message.is_user_message());
        assert!(!message.has_tool_calls());
    }

    #[test]
    fn test_message_with_empty_content_is_invalid() {
        let message = Message::new(
            Uuid::new_v4(),
            MessageRole::User,
            "".to_string(),
            Utc::now(),
            1,
        );

        assert!(!message.is_valid());
    }

    #[test]
    fn test_message_with_tool_calls() {
        let tool_call = ToolCall {
            id: "call_1".to_string(),
            function: "get_weather".to_string(),
            arguments: serde_json::json!({"location": "San Francisco"}),
            result: Some(serde_json::json!({"temperature": "72F"})),
        };

        let message = Message::new(
            Uuid::new_v4(),
            MessageRole::Assistant,
            "I'll check the weather for you.".to_string(),
            Utc::now(),
            1,
        )
        .with_tool_calls(vec![tool_call]);

        assert!(message.has_tool_calls());
        assert!(message.is_assistant_message());
        assert_eq!(message.tool_calls.as_ref().unwrap().len(), 1);
    }

    #[test]
    fn test_message_ordering() {
        let session_id = Uuid::new_v4();
        let timestamp = Utc::now();

        let message1 = Message::new(
            session_id,
            MessageRole::User,
            "First message".to_string(),
            timestamp,
            1,
        );

        let message2 = Message::new(
            session_id,
            MessageRole::Assistant,
            "Second message".to_string(),
            timestamp,
            2,
        );

        assert!(message1 < message2);
    }

    #[test]
    fn test_role_display() {
        assert_eq!(MessageRole::User.to_string(), "User");
        assert_eq!(MessageRole::Assistant.to_string(), "Assistant");
        assert_eq!(MessageRole::System.to_string(), "System");
    }

    #[test]
    fn test_word_count() {
        let message = Message::new(
            Uuid::new_v4(),
            MessageRole::User,
            "Hello world this is a test".to_string(),
            Utc::now(),
            1,
        );

        assert_eq!(message.word_count(), 6);
        assert_eq!(message.content_length(), 26);
    }
}
