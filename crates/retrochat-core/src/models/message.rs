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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub enum MessageType {
    ToolRequest,
    ToolResult,
    Thinking,
    #[default]
    SimpleMessage,
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

impl std::fmt::Display for MessageType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MessageType::ToolRequest => write!(f, "tool_request"),
            MessageType::ToolResult => write!(f, "tool_result"),
            MessageType::Thinking => write!(f, "thinking"),
            MessageType::SimpleMessage => write!(f, "simple_message"),
        }
    }
}

impl std::str::FromStr for MessageType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "tool_request" => Ok(MessageType::ToolRequest),
            "tool_result" => Ok(MessageType::ToolResult),
            "thinking" => Ok(MessageType::Thinking),
            "simple_message" => Ok(MessageType::SimpleMessage),
            _ => Err(format!("Unknown message type: {s}")),
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
    pub metadata: Option<Value>,
    pub sequence_number: u32,
    pub message_type: MessageType,
    pub tool_operation_id: Option<Uuid>,

    // TRANSIENT FIELDS: Used only during import, never persisted to database
    // These fields are populated by parsers and consumed by ImportService to create ToolOperations
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_uses: Option<Vec<ToolUse>>,
    #[serde(skip_serializing_if = "Option::is_none")]
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
            metadata: None,
            sequence_number,
            message_type: MessageType::default(),
            tool_operation_id: None,
            tool_uses: None,
            tool_results: None,
        }
    }

    pub fn with_token_count(mut self, token_count: u32) -> Self {
        self.token_count = Some(token_count);
        self
    }

    pub fn with_metadata(mut self, metadata: Value) -> Self {
        self.metadata = Some(metadata);
        self
    }

    pub fn with_message_type(mut self, message_type: MessageType) -> Self {
        self.message_type = message_type;
        self
    }

    pub fn with_tool_operation(mut self, tool_operation_id: Uuid) -> Self {
        self.tool_operation_id = Some(tool_operation_id);
        self
    }

    /// Set tool_uses (transient field - only used during import)
    pub fn with_tool_uses(mut self, tool_uses: Vec<ToolUse>) -> Self {
        self.tool_uses = Some(tool_uses);
        self
    }

    /// Set tool_results (transient field - only used during import)
    pub fn with_tool_results(mut self, tool_results: Vec<ToolResult>) -> Self {
        self.tool_results = Some(tool_results);
        self
    }

    pub fn is_valid(&self) -> bool {
        !self.content.is_empty()
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

    /// Check if this message is a tool request
    pub fn is_tool_request(&self) -> bool {
        matches!(self.message_type, MessageType::ToolRequest)
    }

    /// Check if this message is a tool result
    pub fn is_tool_result(&self) -> bool {
        matches!(self.message_type, MessageType::ToolResult)
    }

    /// Check if this message is thinking
    pub fn is_thinking(&self) -> bool {
        matches!(self.message_type, MessageType::Thinking)
    }

    /// Check if this message has an associated tool operation
    pub fn has_tool_operation(&self) -> bool {
        self.tool_operation_id.is_some()
    }

    /// Get the tool operation associated with this message
    ///
    /// This retrieves the ToolOperation record from the database
    /// which contains parsed file change metrics and other tool-specific data.
    ///
    /// # Example
    /// ```no_run
    /// # use retrochat::database::ToolOperationRepository;
    /// # async fn example(message: retrochat::models::Message, repo: &ToolOperationRepository) {
    /// if let Some(operation) = message.get_tool_operation(repo).await.unwrap() {
    ///     if operation.is_file_operation() {
    ///         if let Some(file_meta) = &operation.file_metadata {
    ///             println!("File: {:?}, Lines changed: {}", file_meta.file_path, operation.total_line_changes());
    ///         }
    ///     }
    /// }
    /// # }
    /// ```
    pub async fn get_tool_operation(
        &self,
        repo: &crate::database::ToolOperationRepository,
    ) -> anyhow::Result<Option<crate::models::ToolOperation>> {
        if let Some(tool_op_id) = self.tool_operation_id {
            repo.get_by_id(&tool_op_id).await
        } else {
            Ok(None)
        }
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
        assert_eq!(message.message_type, MessageType::SimpleMessage);
        assert!(message.is_valid());
        assert!(message.is_user_message());
        assert!(!message.has_tool_operation());
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
    fn test_message_with_tool_operation() {
        let tool_operation_id = Uuid::new_v4();

        let message = Message::new(
            Uuid::new_v4(),
            MessageRole::Assistant,
            "I'll check the weather for you.".to_string(),
            Utc::now(),
            1,
        )
        .with_message_type(MessageType::ToolRequest)
        .with_tool_operation(tool_operation_id);

        assert!(message.has_tool_operation());
        assert!(message.is_tool_request());
        assert!(message.is_assistant_message());
        assert_eq!(message.tool_operation_id, Some(tool_operation_id));
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
    fn test_message_type_display() {
        assert_eq!(MessageType::ToolRequest.to_string(), "tool_request");
        assert_eq!(MessageType::ToolResult.to_string(), "tool_result");
        assert_eq!(MessageType::SimpleMessage.to_string(), "simple_message");
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

    #[tokio::test]
    async fn test_get_tool_operation() {
        use crate::database::{
            ChatSessionRepository, DatabaseManager, MessageRepository, ToolOperationRepository,
        };
        use crate::models::{ChatSession, Provider, SessionState, ToolOperation};

        let db = DatabaseManager::open_in_memory().await.unwrap();
        let session_repo = ChatSessionRepository::new(&db);
        let message_repo = MessageRepository::new(&db);
        let tool_op_repo = ToolOperationRepository::new(&db);

        // Create session
        let session_id = Uuid::new_v4();
        let mut session = ChatSession::new(
            Provider::ClaudeCode,
            "/test/file.jsonl".to_string(),
            "test_hash".to_string(),
            Utc::now(),
        );
        session.id = session_id;
        session.set_state(SessionState::Imported);
        session_repo.create(&session).await.unwrap();

        // Create tool operation
        let tool_op = ToolOperation::new("tool_1".to_string(), "Write".to_string(), Utc::now())
            .with_file_path("/test.rs".to_string());
        let tool_op_id = tool_op.id;
        tool_op_repo.create(&tool_op).await.unwrap();

        // Create message with tool operation
        let message_id = Uuid::new_v4();
        let mut message = Message::new(
            session_id,
            MessageRole::Assistant,
            "test message".to_string(),
            Utc::now(),
            1,
        )
        .with_message_type(MessageType::ToolRequest)
        .with_tool_operation(tool_op_id);
        message.id = message_id;
        message_repo.create(&message).await.unwrap();

        // Test helper method
        let operation = message.get_tool_operation(&tool_op_repo).await.unwrap();
        assert!(operation.is_some());
        let op = operation.unwrap();
        assert_eq!(op.tool_name, "Write");
        assert!(op.file_metadata.is_some());
        assert_eq!(
            op.file_metadata.as_ref().unwrap().file_path,
            "/test.rs".to_string()
        );
    }

    #[test]
    fn test_message_type_checks() {
        let tool_request_msg = Message::new(
            Uuid::new_v4(),
            MessageRole::Assistant,
            "test".to_string(),
            Utc::now(),
            1,
        )
        .with_message_type(MessageType::ToolRequest);

        let tool_result_msg = Message::new(
            Uuid::new_v4(),
            MessageRole::Assistant,
            "test".to_string(),
            Utc::now(),
            2,
        )
        .with_message_type(MessageType::ToolResult);

        let simple_msg = Message::new(
            Uuid::new_v4(),
            MessageRole::User,
            "test".to_string(),
            Utc::now(),
            3,
        );

        assert!(tool_request_msg.is_tool_request());
        assert!(!tool_request_msg.is_tool_result());

        assert!(!tool_result_msg.is_tool_request());
        assert!(tool_result_msg.is_tool_result());

        assert!(!simple_msg.is_tool_request());
        assert!(!simple_msg.is_tool_result());
    }
}
