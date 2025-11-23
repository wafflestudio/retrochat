use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use uuid::Uuid;

use crate::models::{ChatSession, Message, MessageRole, ToolResult, ToolUse};
use crate::models::{Provider, SessionState};
use crate::parsers::project_inference::ProjectInference;

#[derive(Debug, Serialize, Deserialize)]
pub struct GeminiMessage {
    pub parts: Vec<GeminiPart>,
    pub role: String,
    pub timestamp: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GeminiPart {
    pub text: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GeminiConversation {
    pub conversation_id: Option<String>,
    pub create_time: Option<String>,
    pub update_time: Option<String>,
    pub conversation: Vec<GeminiMessage>,
    pub title: Option<String>,
    pub model_display_name: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GeminiExport {
    pub conversations: Vec<GeminiConversation>,
}

// New structures for the actual Gemini export format
#[derive(Debug, Serialize, Deserialize)]
pub struct GeminiSessionMessage {
    pub id: String,
    pub timestamp: String,
    #[serde(rename = "type")]
    pub message_type: String,
    pub content: String,
    pub thoughts: Option<Vec<GeminiThought>>,
    pub tokens: Option<GeminiTokens>,
    pub model: Option<String>,
    #[serde(rename = "toolCalls")]
    pub tool_calls: Option<Vec<GeminiToolCall>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GeminiThought {
    pub subject: String,
    pub description: String,
    pub timestamp: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GeminiTokens {
    pub input: u32,
    pub output: u32,
    pub cached: u32,
    pub thoughts: u32,
    pub tool: u32,
    pub total: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GeminiToolCall {
    pub id: String,
    pub name: String,
    pub args: Value,
    pub result: Option<Vec<GeminiFunctionResponse>>,
    pub status: Option<String>,
    pub timestamp: Option<String>,
    #[serde(rename = "displayName")]
    pub display_name: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GeminiFunctionResponse {
    #[serde(rename = "functionResponse")]
    pub function_response: GeminiFunctionResponseData,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GeminiFunctionResponseData {
    pub id: String,
    pub name: String,
    pub response: Value,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GeminiSession {
    #[serde(rename = "sessionId")]
    pub session_id: String,
    #[serde(rename = "projectHash")]
    pub project_hash: Option<String>,
    #[serde(rename = "startTime")]
    pub start_time: String,
    #[serde(rename = "lastUpdated")]
    pub last_updated: String,
    pub messages: Vec<GeminiSessionMessage>,
}

// Array format structures (simple message list grouped by session)
#[derive(Debug, Serialize, Deserialize)]
pub struct GeminiArrayMessage {
    #[serde(rename = "sessionId")]
    pub session_id: String,
    #[serde(rename = "messageId")]
    pub message_id: u32,
    #[serde(rename = "type")]
    pub message_type: String,
    pub message: String,
    pub timestamp: String,
}

pub struct GeminiCLIParser {
    file_path: String,
    use_memory_mapping: bool,
}

impl GeminiCLIParser {
    pub fn new(file_path: impl AsRef<Path>) -> Self {
        Self {
            file_path: file_path.as_ref().to_string_lossy().to_string(),
            use_memory_mapping: false,
        }
    }

    pub fn with_memory_mapping(mut self, use_mmap: bool) -> Self {
        self.use_memory_mapping = use_mmap;
        self
    }

    pub async fn parse(&self) -> Result<Vec<(ChatSession, Vec<Message>)>> {
        if self.use_memory_mapping {
            self.parse_with_mmap().await
        } else {
            self.parse_standard().await
        }
    }

    async fn parse_standard(&self) -> Result<Vec<(ChatSession, Vec<Message>)>> {
        let mut file = File::open(&self.file_path)
            .with_context(|| format!("Failed to open file: {}", self.file_path))?;

        let mut content = String::new();
        file.read_to_string(&mut content)
            .with_context(|| "Failed to read file content")?;

        self.parse_content(&content).await
    }

    async fn parse_with_mmap(&self) -> Result<Vec<(ChatSession, Vec<Message>)>> {
        // For now, fallback to standard parsing
        // TODO: Implement actual memory mapping using memmap2 crate
        self.parse_standard().await
    }

    async fn parse_content(&self, content: &str) -> Result<Vec<(ChatSession, Vec<Message>)>> {
        // Check if filename matches session-*.json pattern
        if let Some(session_id_from_filename) = self.extract_session_id_from_filename() {
            // Try to parse as array of session messages (new format)
            if let Ok(messages) = serde_json::from_str::<Vec<GeminiSessionMessage>>(content) {
                return self
                    .parse_messages_with_filename_session_id(messages, session_id_from_filename)
                    .await;
            }
        }

        // Try to parse as the old session format (with sessionId field)
        if let Ok(session) = serde_json::from_str::<GeminiSession>(content) {
            return self.parse_session_format(session).await;
        }

        // Try to parse as array of message objects
        if let Ok(array_messages) = serde_json::from_str::<Vec<GeminiArrayMessage>>(content) {
            return self.parse_array_format(array_messages).await;
        }

        // Fallback to old export format
        let gemini_export: GeminiExport =
            serde_json::from_str(content).with_context(|| "Failed to parse Gemini export JSON")?;

        if gemini_export.conversations.is_empty() {
            return Err(anyhow!("No conversations found in Gemini export"));
        }

        let mut results = Vec::new();

        for (index, conversation) in gemini_export.conversations.iter().enumerate() {
            match self.convert_conversation(conversation, index).await {
                Ok((session, messages)) => results.push((session, messages)),
                Err(e) => {
                    // Log error but continue with other conversations
                    tracing::warn!(error = %e, index = index, "Failed to parse conversation");
                    continue;
                }
            }
        }

        if results.is_empty() {
            return Err(anyhow!("No valid conversations could be parsed"));
        }

        Ok(results)
    }

    fn extract_session_id_from_filename(&self) -> Option<String> {
        let path = Path::new(&self.file_path);
        let filename = path.file_stem()?.to_str()?;

        // Check if filename starts with "session-"
        if filename.starts_with("session-") {
            Some(filename.to_string())
        } else {
            None
        }
    }

    async fn parse_messages_with_filename_session_id(
        &self,
        messages: Vec<GeminiSessionMessage>,
        session_id_str: String,
    ) -> Result<Vec<(ChatSession, Vec<Message>)>> {
        if messages.is_empty() {
            return Err(anyhow!("No messages found in Gemini session file"));
        }

        // Generate UUID from session ID string
        let session_id = self.generate_uuid_from_string(&session_id_str);

        // Extract timestamps from messages for start/end time
        let timestamps: Vec<DateTime<Utc>> = messages
            .iter()
            .filter_map(|msg| self.parse_timestamp(&msg.timestamp).ok())
            .collect();

        let start_time = timestamps.iter().min().cloned().unwrap_or_else(Utc::now);
        let end_time = timestamps.iter().max().cloned();

        let file_hash = self.calculate_file_hash()?;

        let mut chat_session = ChatSession::new(
            Provider::GeminiCLI,
            self.file_path.clone(),
            file_hash,
            start_time,
        );

        chat_session.id = session_id;
        if let Some(end) = end_time {
            if end != start_time {
                chat_session = chat_session.with_end_time(end);
            }
        }

        // Extract project_hash from the first message if available
        // The session_id_str format is typically: session-{timestamp}-{project_hash_prefix}
        // We'll use the last part after the last hyphen as project identifier
        if let Some(project_identifier) = session_id_str.rsplit('-').next() {
            chat_session = chat_session.with_project(project_identifier.to_string());
        } else {
            // Fallback to file path inference
            let project_inference = ProjectInference::new(&self.file_path);
            if let Some(project_name) = project_inference.infer_project_name() {
                chat_session = chat_session.with_project(project_name);
            }
        }

        // Convert messages
        let mut converted_messages = Vec::new();
        let mut total_tokens = 0u32;

        for (index, session_message) in messages.iter().enumerate() {
            let message = self.convert_session_message(session_message, session_id, index + 1)?;

            if let Some(token_count) = message.token_count {
                total_tokens += token_count;
            }

            converted_messages.push(message);
        }

        chat_session.message_count = converted_messages.len() as u32;
        if total_tokens > 0 {
            chat_session = chat_session.with_token_count(total_tokens);
        }

        chat_session.set_state(SessionState::Imported);

        Ok(vec![(chat_session, converted_messages)])
    }

    async fn parse_session_format(
        &self,
        session: GeminiSession,
    ) -> Result<Vec<(ChatSession, Vec<Message>)>> {
        if session.messages.is_empty() {
            return Err(anyhow!("No messages found in Gemini session"));
        }

        let session_id = if let Ok(uuid) = Uuid::parse_str(&session.session_id) {
            uuid
        } else {
            self.generate_uuid_from_string(&session.session_id)
        };

        let start_time = self.parse_timestamp(&session.start_time)?;
        let end_time = self.parse_timestamp(&session.last_updated)?;
        let file_hash = self.calculate_file_hash()?;

        let mut chat_session = ChatSession::new(
            Provider::GeminiCLI,
            self.file_path.clone(),
            file_hash,
            start_time,
        );

        chat_session.id = session_id;
        chat_session = chat_session.with_end_time(end_time);

        // Use project_hash as project name if available
        if let Some(project_hash) = &session.project_hash {
            // Use the full project_hash prefix as the project name
            chat_session = chat_session.with_project(project_hash.clone());
        } else {
            // Fallback to file path inference
            let project_inference = ProjectInference::new(&self.file_path);
            if let Some(project_name) = project_inference.infer_project_name() {
                chat_session = chat_session.with_project(project_name);
            }
        }

        let mut messages = Vec::new();
        let mut total_tokens = 0u32;

        for (index, session_message) in session.messages.iter().enumerate() {
            let message = self.convert_session_message(session_message, session_id, index + 1)?;

            if let Some(token_count) = message.token_count {
                total_tokens += token_count;
            }

            messages.push(message);
        }

        chat_session.message_count = messages.len() as u32;
        if total_tokens > 0 {
            chat_session = chat_session.with_token_count(total_tokens);
        }

        chat_session.set_state(SessionState::Imported);

        Ok(vec![(chat_session, messages)])
    }

    async fn parse_array_format(
        &self,
        array_messages: Vec<GeminiArrayMessage>,
    ) -> Result<Vec<(ChatSession, Vec<Message>)>> {
        use std::collections::HashMap;

        if array_messages.is_empty() {
            return Err(anyhow!("No messages found in array format"));
        }

        // Group messages by session_id
        let mut sessions_map: HashMap<String, Vec<&GeminiArrayMessage>> = HashMap::new();
        for msg in &array_messages {
            sessions_map
                .entry(msg.session_id.clone())
                .or_default()
                .push(msg);
        }

        let mut results = Vec::new();

        for (session_id_str, session_messages) in sessions_map {
            // Parse or generate session UUID
            let session_id = if let Ok(uuid) = Uuid::parse_str(&session_id_str) {
                uuid
            } else {
                self.generate_uuid_from_string(&session_id_str)
            };

            // Find start and end times from messages
            let timestamps: Vec<DateTime<Utc>> = session_messages
                .iter()
                .filter_map(|msg| self.parse_timestamp(&msg.timestamp).ok())
                .collect();

            let start_time = timestamps.iter().min().cloned().unwrap_or_else(Utc::now);
            let end_time = timestamps.iter().max().cloned();

            let file_hash = self.calculate_file_hash()?;

            let mut chat_session = ChatSession::new(
                Provider::GeminiCLI,
                self.file_path.clone(),
                file_hash,
                start_time,
            );

            chat_session.id = session_id;
            if let Some(end) = end_time {
                if end != start_time {
                    chat_session = chat_session.with_end_time(end);
                }
            }

            // Use project inference to determine project name from file path
            let project_inference = ProjectInference::new(&self.file_path);
            if let Some(project_name) = project_inference.infer_project_name() {
                chat_session = chat_session.with_project(project_name);
            }

            // Convert messages
            let mut messages = Vec::new();
            let mut total_tokens = 0u32;

            // Sort by messageId to ensure correct order
            let mut sorted_messages = session_messages.clone();
            sorted_messages.sort_by_key(|m| m.message_id);

            for (index, array_msg) in sorted_messages.iter().enumerate() {
                let role = match array_msg.message_type.as_str() {
                    "user" => MessageRole::User,
                    "gemini" | "assistant" => MessageRole::Assistant,
                    "system" => MessageRole::System,
                    _ => MessageRole::User, // Default to user for unknown types
                };

                let timestamp = self
                    .parse_timestamp(&array_msg.timestamp)
                    .unwrap_or(start_time);

                let message_id = self.generate_uuid_from_string(&format!(
                    "{}-msg-{}",
                    session_id, array_msg.message_id
                ));

                let mut message = Message::new(
                    session_id,
                    role,
                    array_msg.message.clone(),
                    timestamp,
                    (index + 1) as u32,
                );

                message.id = message_id;

                // Estimate token count based on content length
                let estimated_tokens = (message.content.len() / 4) as u32;
                if estimated_tokens > 0 {
                    message = message.with_token_count(estimated_tokens);
                    total_tokens += estimated_tokens;
                }

                messages.push(message);
            }

            chat_session.message_count = messages.len() as u32;
            if total_tokens > 0 {
                chat_session = chat_session.with_token_count(total_tokens);
            }

            chat_session.set_state(SessionState::Imported);

            results.push((chat_session, messages));
        }

        if results.is_empty() {
            return Err(anyhow!("No valid sessions could be parsed"));
        }

        Ok(results)
    }

    /// Normalize Gemini tool names to standard retrochat tool names
    fn normalize_tool_name(gemini_name: &str) -> String {
        match gemini_name {
            "replace" => "Edit".to_string(),
            "run_shell_command" => "Bash".to_string(),
            "read_file" => "Read".to_string(),
            "write_file" | "write_to_file" => "Write".to_string(),
            _ => {
                // Capitalize first letter for unknown tools
                let mut chars = gemini_name.chars();
                match chars.next() {
                    None => String::new(),
                    Some(first) => first.to_uppercase().chain(chars).collect(),
                }
            }
        }
    }

    /// Extract tool operations from Gemini toolCalls
    /// Returns (tool_uses, tool_results)
    fn extract_tool_operations(
        &self,
        tool_calls: &[GeminiToolCall],
    ) -> (Vec<ToolUse>, Vec<ToolResult>) {
        let mut tool_uses = Vec::new();
        let mut tool_results = Vec::new();

        for tool_call in tool_calls {
            // Create ToolUse
            let normalized_name = Self::normalize_tool_name(&tool_call.name);
            tool_uses.push(ToolUse {
                id: tool_call.id.clone(),
                name: normalized_name.clone(),
                input: tool_call.args.clone(),
                raw: serde_json::to_value(tool_call).unwrap_or(Value::Null),
            });

            // Extract ToolResult if available
            if let Some(results) = &tool_call.result {
                for func_response in results {
                    let response_data = &func_response.function_response;

                    // Extract content from response
                    let content = if let Some(output) = response_data.response.get("output") {
                        output.as_str().unwrap_or_default().to_string()
                    } else {
                        serde_json::to_string(&response_data.response).unwrap_or_default()
                    };

                    // Check if it's an error based on status
                    let is_error = tool_call
                        .status
                        .as_ref()
                        .map(|s| s != "success")
                        .unwrap_or(false);

                    tool_results.push(ToolResult {
                        tool_use_id: tool_call.id.clone(),
                        content,
                        is_error,
                        details: Some(response_data.response.clone()),
                        raw: serde_json::to_value(func_response).unwrap_or(Value::Null),
                    });
                }
            }
        }

        (tool_uses, tool_results)
    }

    fn convert_session_message(
        &self,
        session_message: &GeminiSessionMessage,
        session_id: Uuid,
        sequence: usize,
    ) -> Result<Message> {
        let role = match session_message.message_type.as_str() {
            "user" => MessageRole::User,
            "gemini" => MessageRole::Assistant,
            "system" => MessageRole::System,
            _ => {
                return Err(anyhow!(
                    "Unknown message type: {}",
                    session_message.message_type
                ))
            }
        };

        if session_message.content.is_empty() {
            return Err(anyhow!("Message has no content"));
        }

        let timestamp = self.parse_timestamp(&session_message.timestamp)?;

        // Generate a deterministic UUID for the message
        let message_id = if let Ok(uuid) = Uuid::parse_str(&session_message.id) {
            uuid
        } else {
            self.generate_uuid_from_string(&format!("{session_id}-msg-{}", session_message.id))
        };

        let mut message = Message::new(
            session_id,
            role,
            session_message.content.clone(),
            timestamp,
            sequence as u32,
        );

        message.id = message_id;

        // Extract tool operations if present
        if let Some(tool_calls) = &session_message.tool_calls {
            let (tool_uses, tool_results) = self.extract_tool_operations(tool_calls);
            message.tool_uses = Some(tool_uses);
            message.tool_results = Some(tool_results);
        }

        // Use actual token count if available
        if let Some(tokens) = &session_message.tokens {
            message = message.with_token_count(tokens.total);
        } else {
            // Estimate token count based on content length
            let estimated_tokens = (message.content.len() / 4) as u32; // Rough estimate: 4 chars per token
            if estimated_tokens > 0 {
                message = message.with_token_count(estimated_tokens);
            }
        }

        Ok(message)
    }

    async fn convert_conversation(
        &self,
        conversation: &GeminiConversation,
        index: usize,
    ) -> Result<(ChatSession, Vec<Message>)> {
        let session_id = if let Some(conv_id) = &conversation.conversation_id {
            // Try to parse as UUID, fallback to generating from hash
            Uuid::parse_str(conv_id)
                .unwrap_or_else(|_| self.generate_uuid_from_string(&format!("{conv_id}-{index}")))
        } else {
            // Generate UUID from file path and index
            self.generate_uuid_from_string(&format!("{}-conversation-{}", self.file_path, index))
        };

        let start_time = if let Some(create_time) = &conversation.create_time {
            self.parse_timestamp(create_time)?
        } else {
            // Use first message timestamp or current time
            if let Some(first_msg) = conversation.conversation.first() {
                if let Some(ts) = &first_msg.timestamp {
                    self.parse_timestamp(ts)?
                } else {
                    Utc::now()
                }
            } else {
                Utc::now()
            }
        };

        let end_time = if let Some(update_time) = &conversation.update_time {
            Some(self.parse_timestamp(update_time)?)
        } else {
            // Use last message timestamp
            if let Some(last_msg) = conversation.conversation.last() {
                if let Some(ts) = &last_msg.timestamp {
                    Some(self.parse_timestamp(ts)?)
                } else {
                    None
                }
            } else {
                None
            }
        };

        let file_hash = self.calculate_file_hash()?;

        let mut chat_session = ChatSession::new(
            Provider::GeminiCLI,
            self.file_path.clone(),
            file_hash,
            start_time,
        );

        chat_session.id = session_id;

        if let Some(end) = end_time {
            chat_session = chat_session.with_end_time(end);
        }

        if let Some(title) = &conversation.title {
            chat_session = chat_session.with_project(title.clone());
        } else {
            // Use project inference to determine project name from file path
            let project_inference = ProjectInference::new(&self.file_path);
            if let Some(project_name) = project_inference.infer_project_name() {
                chat_session = chat_session.with_project(project_name);
            }
        }

        let mut messages = Vec::new();
        let mut total_tokens = 0u32;

        for (msg_index, gemini_message) in conversation.conversation.iter().enumerate() {
            let message = self.convert_message(gemini_message, session_id, msg_index + 1)?;

            if let Some(token_count) = message.token_count {
                total_tokens += token_count;
            }

            messages.push(message);
        }

        chat_session.message_count = messages.len() as u32;
        if total_tokens > 0 {
            chat_session = chat_session.with_token_count(total_tokens);
        }

        chat_session.set_state(SessionState::Imported);

        Ok((chat_session, messages))
    }

    fn convert_message(
        &self,
        gemini_message: &GeminiMessage,
        session_id: Uuid,
        sequence: usize,
    ) -> Result<Message> {
        let role = match gemini_message.role.as_str() {
            "user" => MessageRole::User,
            "model" => MessageRole::Assistant,
            "system" => MessageRole::System,
            _ => return Err(anyhow!("Unknown message role: {}", gemini_message.role)),
        };

        // Combine all parts into a single content string
        let content = gemini_message
            .parts
            .iter()
            .map(|part| part.text.clone())
            .collect::<Vec<_>>()
            .join(" ");

        if content.is_empty() {
            return Err(anyhow!("Message has no content"));
        }

        let timestamp = if let Some(ts) = &gemini_message.timestamp {
            self.parse_timestamp(ts)?
        } else {
            // Use a timestamp based on sequence for ordering
            Utc::now() + chrono::Duration::seconds(sequence as i64)
        };

        // Generate a deterministic UUID for the message
        let message_id = self.generate_uuid_from_string(&format!("{session_id}-msg-{sequence}"));

        let mut message = Message::new(session_id, role, content, timestamp, sequence as u32);

        message.id = message_id;

        // Estimate token count based on content length
        let estimated_tokens = (message.content.len() / 4) as u32; // Rough estimate: 4 chars per token
        if estimated_tokens > 0 {
            message = message.with_token_count(estimated_tokens);
        }

        Ok(message)
    }

    fn parse_timestamp(&self, timestamp_str: &str) -> Result<DateTime<Utc>> {
        // Try different timestamp formats that Gemini might use
        let formats = [
            "%Y-%m-%dT%H:%M:%S%.fZ",
            "%Y-%m-%dT%H:%M:%SZ",
            "%Y-%m-%dT%H:%M:%S%.f%z",
            "%Y-%m-%dT%H:%M:%S%z",
            "%Y-%m-%d %H:%M:%S UTC",
            "%Y-%m-%d %H:%M:%S",
        ];

        for format in &formats {
            if let Ok(dt) = DateTime::parse_from_str(timestamp_str, format) {
                return Ok(dt.with_timezone(&Utc));
            }
        }

        // Try parsing as RFC3339
        if let Ok(dt) = DateTime::parse_from_rfc3339(timestamp_str) {
            return Ok(dt.with_timezone(&Utc));
        }

        // Try parsing as Unix timestamp (seconds)
        if let Ok(timestamp) = timestamp_str.parse::<i64>() {
            if let Some(dt) = DateTime::from_timestamp(timestamp, 0) {
                return Ok(dt);
            }
        }

        // Try parsing as Unix timestamp (milliseconds)
        if let Ok(timestamp_ms) = timestamp_str.parse::<i64>() {
            if let Some(dt) = DateTime::from_timestamp_millis(timestamp_ms) {
                return Ok(dt);
            }
        }

        // Fallback: try to parse as Utc directly
        if let Ok(dt) = timestamp_str.parse::<DateTime<Utc>>() {
            return Ok(dt);
        }

        Err(anyhow!("Unable to parse timestamp: {timestamp_str}"))
    }

    fn calculate_file_hash(&self) -> Result<String> {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let metadata = std::fs::metadata(&self.file_path)
            .with_context(|| format!("Failed to get file metadata: {}", self.file_path))?;

        let mut hasher = DefaultHasher::new();
        self.file_path.hash(&mut hasher);
        metadata.len().hash(&mut hasher);
        if let Ok(modified) = metadata.modified() {
            if let Ok(duration) = modified.duration_since(std::time::UNIX_EPOCH) {
                duration.as_secs().hash(&mut hasher);
            }
        }

        Ok(format!("{:x}", hasher.finish()))
    }

    fn generate_uuid_from_string(&self, input: &str) -> Uuid {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        input.hash(&mut hasher);
        let hash = hasher.finish();

        // Create a deterministic UUID from the hash
        let bytes = [
            (hash >> 56) as u8,
            (hash >> 48) as u8,
            (hash >> 40) as u8,
            (hash >> 32) as u8,
            (hash >> 24) as u8,
            (hash >> 16) as u8,
            (hash >> 8) as u8,
            hash as u8,
            (hash >> 56) as u8,
            (hash >> 48) as u8,
            (hash >> 40) as u8,
            (hash >> 32) as u8,
            (hash >> 24) as u8,
            (hash >> 16) as u8,
            (hash >> 8) as u8,
            hash as u8,
        ];

        Uuid::from_bytes(bytes)
    }

    /// Check if the filename matches Gemini's expected format (session-*.json pattern)
    pub fn accepts_filename(file_path: impl AsRef<Path>) -> bool {
        let path = file_path.as_ref();

        // Get the file stem (filename without extension)
        if let Some(file_stem) = path.file_stem().and_then(|s| s.to_str()) {
            // Gemini only accepts files starting with "session-"
            if file_stem.starts_with("session-") {
                return true;
            }
        }

        false
    }

    pub fn is_valid_file(file_path: impl AsRef<Path>) -> bool {
        let path = file_path.as_ref();

        // Check file extension
        if let Some(extension) = path.extension() {
            if extension != "json" {
                return false;
            }
        } else {
            return false;
        }

        // Check if file exists and is readable
        if !path.exists() || !path.is_file() {
            return false;
        }

        // Check filename filter first - Gemini only accepts files starting with "session-"
        if !Self::accepts_filename(path) {
            return false;
        }

        // Check if filename matches session-*.json pattern
        let filename_matches_pattern = path
            .file_stem()
            .and_then(|s| s.to_str())
            .map(|s| s.starts_with("session-"))
            .unwrap_or(false);

        // Try to read and parse the file to see if it looks like Gemini format
        if let Ok(mut file) = File::open(path) {
            let mut content = String::new();
            if file.read_to_string(&mut content).is_ok() {
                if let Ok(parsed) = serde_json::from_str::<Value>(&content) {
                    // Check for new session format (array of messages with filename pattern)
                    if filename_matches_pattern && parsed.is_array() {
                        if let Some(arr) = parsed.as_array() {
                            // Check if first element has message structure
                            if let Some(first) = arr.first() {
                                if first.get("id").is_some()
                                    && first.get("type").is_some()
                                    && first.get("content").is_some()
                                {
                                    return true;
                                }
                            }
                        }
                    }
                    // Check for session format with sessionId field
                    if parsed.get("sessionId").is_some() && parsed.get("messages").is_some() {
                        return true;
                    }
                    // Check for old Gemini export format
                    if parsed.get("conversations").is_some() {
                        return true;
                    }
                    // Also check if it's a single conversation object
                    if parsed.get("conversation").is_some()
                        || parsed.get("conversation_id").is_some()
                    {
                        return true;
                    }
                }
            }
        }

        false
    }

    pub async fn parse_streaming<F>(&self, mut callback: F) -> Result<()>
    where
        F: FnMut(ChatSession, Message) -> Result<()>,
    {
        let sessions = self.parse().await?;

        for (session, messages) in sessions {
            for message in messages {
                callback(session.clone(), message)?;
            }
        }

        Ok(())
    }

    pub fn get_conversation_count(&self) -> Result<usize> {
        let mut file = File::open(&self.file_path)
            .with_context(|| format!("Failed to open file: {}", self.file_path))?;

        let mut content = String::new();
        file.read_to_string(&mut content)
            .with_context(|| "Failed to read file content")?;

        let gemini_export: GeminiExport =
            serde_json::from_str(&content).with_context(|| "Failed to parse Gemini export JSON")?;

        Ok(gemini_export.conversations.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[tokio::test]
    async fn test_parse_gemini_conversation() {
        let mut temp_file = NamedTempFile::with_suffix(".json").unwrap();
        let sample_data = r#"{"conversations":[{"conversation_id":"test-123","create_time":"2024-01-01T10:00:00Z","update_time":"2024-01-01T11:00:00Z","title":"Test Chat","conversation":[{"parts":[{"text":"Hello"}],"role":"user","timestamp":"2024-01-01T10:00:00Z"},{"parts":[{"text":"Hi there!"}],"role":"model","timestamp":"2024-01-01T10:01:00Z"}]}]}"#;

        temp_file.write_all(sample_data.as_bytes()).unwrap();

        let parser = GeminiCLIParser::new(temp_file.path());
        let result = parser.parse().await;

        assert!(result.is_ok());
        let sessions = result.unwrap();

        assert_eq!(sessions.len(), 1);

        let (session, messages) = &sessions[0];
        assert_eq!(session.provider, Provider::GeminiCLI);
        assert_eq!(session.message_count, 2);
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].role, MessageRole::User);
        assert_eq!(messages[1].role, MessageRole::Assistant);
    }

    #[test]
    fn test_is_valid_file() {
        let mut temp_file = NamedTempFile::with_suffix(".json").unwrap();
        let sample_data = r#"{"conversations":[]}"#;
        temp_file.write_all(sample_data.as_bytes()).unwrap();

        assert!(GeminiCLIParser::is_valid_file(temp_file.path()));
    }

    #[test]
    fn test_is_invalid_file() {
        let mut temp_file = NamedTempFile::with_suffix(".txt").unwrap();
        temp_file.write_all(b"not json").unwrap();

        assert!(!GeminiCLIParser::is_valid_file(temp_file.path()));
    }

    #[tokio::test]
    async fn test_get_conversation_count() {
        let mut temp_file = NamedTempFile::with_suffix(".json").unwrap();
        let sample_data = r#"{"conversations":[{"conversation":[]},{"conversation":[]}]}"#;
        temp_file.write_all(sample_data.as_bytes()).unwrap();

        let parser = GeminiCLIParser::new(temp_file.path());
        let count = parser.get_conversation_count().unwrap();

        assert_eq!(count, 2);
    }
}
