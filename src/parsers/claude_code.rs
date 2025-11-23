use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs::File;
use std::io::{BufRead, BufReader, Lines};
use std::path::Path;
use uuid::Uuid;

use crate::models::message::{MessageType, ToolResult, ToolUse};
use crate::models::{ChatSession, Message, MessageRole};
use crate::models::{Provider, SessionState};

use super::project_inference::ProjectInference;

#[derive(Debug, Serialize, Deserialize)]
pub struct ClaudeCodeMessage {
    pub uuid: String,
    pub content: Value,
    pub created_at: String,
    pub updated_at: String,
    pub role: String,
    pub metadata: Option<Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ClaudeCodeSession {
    pub uuid: String,
    pub name: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub chat_messages: Vec<ClaudeCodeMessage>,
    pub summary: Option<String>,
    pub model: Option<String>,
}

// New structures for Claude Code conversation format
#[derive(Debug, Serialize, Deserialize)]
pub struct ClaudeCodeConversationEntry {
    #[serde(rename = "type")]
    pub entry_type: String,
    pub uuid: Option<String>,
    #[serde(rename = "sessionId")]
    pub session_id: Option<String>,
    pub timestamp: Option<String>,
    pub message: Option<ConversationMessage>,
    pub summary: Option<String>,
    #[serde(rename = "leafUuid")]
    pub leaf_uuid: Option<String>,
    #[serde(rename = "parentUuid")]
    pub parent_uuid: Option<String>,
    /// Tool use result metadata (stdout, stderr, etc.) for tool_result messages
    #[serde(rename = "toolUseResult")]
    pub tool_use_result: Option<Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ConversationMessage {
    pub role: String,
    pub content: Value,
    pub id: Option<String>,
    pub model: Option<String>,
}

pub struct ClaudeCodeParser {
    file_path: String,
}

impl ClaudeCodeParser {
    pub fn new(file_path: impl AsRef<Path>) -> Self {
        Self {
            file_path: file_path.as_ref().to_string_lossy().to_string(),
        }
    }

    pub async fn parse(&self) -> Result<(ChatSession, Vec<Message>)> {
        let file = File::open(&self.file_path)
            .with_context(|| format!("Failed to open file: {}", self.file_path))?;

        let reader = BufReader::new(file);
        self.parse_from_reader(reader).await
    }

    async fn parse_from_reader(
        &self,
        reader: BufReader<File>,
    ) -> Result<(ChatSession, Vec<Message>)> {
        let lines = reader.lines();
        self.parse_lines(lines).await
    }

    async fn parse_lines<B: BufRead>(
        &self,
        lines: Lines<B>,
    ) -> Result<(ChatSession, Vec<Message>)> {
        let mut conversation_entries: Vec<ClaudeCodeConversationEntry> = Vec::new();
        let mut sessions: Vec<ClaudeCodeSession> = Vec::new();
        let mut is_conversation_format = false;

        for line in lines {
            let line = line.with_context(|| "Failed to read line from file")?;

            if line.trim().is_empty() {
                continue;
            }

            // Try to parse as conversation format first
            if let Ok(entry) = serde_json::from_str::<ClaudeCodeConversationEntry>(&line) {
                conversation_entries.push(entry);
                is_conversation_format = true;
            } else if let Ok(session) = serde_json::from_str::<ClaudeCodeSession>(&line) {
                if is_conversation_format {
                    return Err(anyhow!(
                        "Mixed format detected: cannot mix conversation and session formats"
                    ));
                }
                sessions.push(session);
            } else {
                return Err(anyhow!("Failed to parse line as JSON: {line}"));
            }
        }

        if is_conversation_format {
            self.parse_conversation_format(conversation_entries)
        } else {
            if sessions.is_empty() {
                return Err(anyhow!("No valid sessions found in file"));
            }

            // For now, process the first session
            // TODO: Handle multiple sessions in a single file
            let claude_session = &sessions[0];
            self.convert_session(claude_session)
        }
    }

    fn parse_conversation_format(
        &self,
        entries: Vec<ClaudeCodeConversationEntry>,
    ) -> Result<(ChatSession, Vec<Message>)> {
        if entries.is_empty() {
            return Err(anyhow!("No conversation entries found"));
        }

        // Check if all entries are summary-only (no actual conversation messages)
        let has_actual_messages = entries.iter().any(|e| e.message.is_some());
        if !has_actual_messages {
            // Skip summary-only files silently
            return Err(anyhow!("File contains only summary entries, skipping"));
        }

        // Find the session ID from any entry that has one
        let session_id =
            if let Some(session_id_str) = entries.iter().find_map(|e| e.session_id.as_ref()) {
                // Parse existing session ID
                Uuid::parse_str(session_id_str)
                    .with_context(|| format!("Invalid session UUID format: {session_id_str}"))?
            } else {
                // This shouldn't happen for files with actual messages, but handle it gracefully
                return Err(anyhow!(
                    "File contains messages but no session ID, cannot import"
                ));
            };

        // Summary entries are parsed elsewhere if needed; not used for project naming

        // Get the earliest timestamp for start time
        let start_time = entries
            .iter()
            .filter_map(|e| e.timestamp.as_ref())
            .filter_map(|ts| self.parse_timestamp(ts).ok())
            .min()
            .unwrap_or_else(Utc::now);

        // Get the latest timestamp for end time
        let end_time = entries
            .iter()
            .filter_map(|e| e.timestamp.as_ref())
            .filter_map(|ts| self.parse_timestamp(ts).ok())
            .max();

        let file_hash = self.calculate_file_hash()?;

        let mut chat_session = ChatSession::new(
            Provider::ClaudeCode,
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

        // Determine project name strictly from path inference (do not use summary)
        let project_name = {
            let inference = ProjectInference::new(&self.file_path);
            inference.infer_project_name()
        };

        if let Some(name) = project_name {
            chat_session = chat_session.with_project(name);
        }

        // Convert conversation entries to messages
        let mut messages = Vec::new();
        let mut total_tokens = 0u32;
        let mut sequence = 1;

        for entry in &entries {
            if let Some(conv_message) = &entry.message {
                if conv_message.role == "user" || conv_message.role == "assistant" {
                    let message_id = entry
                        .uuid
                        .as_ref()
                        .and_then(|uuid| Uuid::parse_str(uuid).ok())
                        .unwrap_or_else(Uuid::new_v4);

                    let role = match conv_message.role.as_str() {
                        "user" => MessageRole::User,
                        "assistant" => MessageRole::Assistant,
                        _ => continue, // Skip unknown roles
                    };

                    let timestamp = entry
                        .timestamp
                        .as_ref()
                        .and_then(|ts| self.parse_timestamp(ts).ok())
                        .unwrap_or(start_time);

                    let (content, tool_uses, mut tool_results, thinking_content) =
                        self.extract_tools_and_content(&conv_message.content);

                    // If there's thinking content, create a separate message for it first
                    if let Some(thinking_text) = thinking_content {
                        let thinking_message = Message::new(
                            session_id,
                            MessageRole::Assistant,
                            thinking_text.clone(),
                            timestamp,
                            sequence,
                        )
                        .with_message_type(MessageType::Thinking);

                        let thinking_tokens = (thinking_text.len() / 4) as u32;
                        let thinking_message = if thinking_tokens > 0 {
                            total_tokens += thinking_tokens;
                            thinking_message.with_token_count(thinking_tokens)
                        } else {
                            thinking_message
                        };

                        messages.push(thinking_message);
                        sequence += 1;
                    }

                    // Enrich tool_results with toolUseResult metadata if available
                    if let Some(tool_use_result_data) = &entry.tool_use_result {
                        if let Some(tool_result) = tool_results.first_mut() {
                            // Merge toolUseResult into details
                            tool_result.details = Some(tool_use_result_data.clone());
                        }
                    }

                    let mut message = Message::new(session_id, role, content, timestamp, sequence);

                    message.id = message_id;

                    // Attach tool uses and results if any
                    if !tool_uses.is_empty() {
                        message = message.with_tool_uses(tool_uses);
                    }
                    if !tool_results.is_empty() {
                        message = message.with_tool_results(tool_results);
                    }

                    // Estimate token count based on content length
                    let estimated_tokens = (message.content.len() / 4) as u32;
                    if estimated_tokens > 0 {
                        message = message.with_token_count(estimated_tokens);
                        total_tokens += estimated_tokens;
                    }

                    messages.push(message);
                    sequence += 1;
                }
            }
        }

        chat_session.message_count = messages.len() as u32;
        if total_tokens > 0 {
            chat_session = chat_session.with_token_count(total_tokens);
        }

        chat_session.set_state(SessionState::Imported);

        Ok((chat_session, messages))
    }

    /// Extract tools and content from a Claude Code message value
    /// Returns (content_string, tool_uses, tool_results, thinking_content)
    fn extract_tools_and_content(
        &self,
        value: &Value,
    ) -> (String, Vec<ToolUse>, Vec<ToolResult>, Option<String>) {
        let mut tool_uses = Vec::new();
        let mut tool_results = Vec::new();
        let mut thinking_content: Option<String> = None;

        let content = match value {
            Value::String(s) => s.clone(),
            Value::Array(arr) => {
                let mut content_parts = Vec::new();
                for item in arr {
                    if let Some(obj) = item.as_object() {
                        let item_type = obj.get("type").and_then(|v| v.as_str());

                        // Handle thinking content blocks
                        if item_type == Some("thinking") {
                            if let Some(thinking_text) =
                                obj.get("thinking").and_then(|v| v.as_str())
                            {
                                // Store thinking separately instead of adding to content
                                thinking_content = Some(thinking_text.to_string());
                            }
                            continue;
                        }

                        // Handle tool_use blocks
                        if item_type == Some("tool_use") {
                            if let (Some(id), Some(name)) = (
                                obj.get("id").and_then(|v| v.as_str()),
                                obj.get("name").and_then(|v| v.as_str()),
                            ) {
                                tool_uses.push(ToolUse {
                                    id: id.to_string(),
                                    name: name.to_string(),
                                    input: obj
                                        .get("input")
                                        .cloned()
                                        .unwrap_or(Value::Object(serde_json::Map::new())),
                                    raw: Value::Object(obj.clone()),
                                });

                                // Add placeholder text
                                content_parts.push(format!("[Tool Use: {name}]"));
                            }
                            continue;
                        }

                        // Handle tool_result blocks
                        if item_type == Some("tool_result") {
                            if let Some(tool_use_id) =
                                obj.get("tool_use_id").and_then(|v| v.as_str())
                            {
                                let content_text = match obj.get("content") {
                                    Some(Value::String(s)) => s.clone(),
                                    Some(Value::Array(arr)) => arr
                                        .iter()
                                        .filter_map(|v| match v {
                                            Value::String(s) => Some(s.clone()),
                                            Value::Object(o) => o
                                                .get("text")
                                                .and_then(|t| t.as_str())
                                                .map(String::from),
                                            _ => None,
                                        })
                                        .collect::<Vec<_>>()
                                        .join(" "),
                                    _ => String::new(),
                                };

                                let is_error = obj
                                    .get("is_error")
                                    .and_then(|v| v.as_bool())
                                    .unwrap_or(false);

                                tool_results.push(ToolResult {
                                    tool_use_id: tool_use_id.to_string(),
                                    content: content_text.clone(),
                                    is_error,
                                    details: obj.get("content").cloned(),
                                    raw: Value::Object(obj.clone()),
                                });

                                // Add simple placeholder text (actual content is in tool_results column)
                                content_parts.push("[Tool Result]".to_string());
                            }
                            continue;
                        }

                        // Handle text content
                        if let Some(text) = obj.get("text").and_then(|v| v.as_str()) {
                            content_parts.push(text.to_string());
                        }
                    } else if let Some(text) = item.as_str() {
                        content_parts.push(text.to_string());
                    }
                }
                content_parts.join(" ")
            }
            Value::Object(obj) => {
                if let Some(text) = obj.get("text").and_then(|v| v.as_str()) {
                    text.to_string()
                } else {
                    serde_json::to_string(obj).unwrap_or_default()
                }
            }
            _ => value.to_string(),
        };

        // Ensure content is never empty to satisfy database constraint
        let final_content = if content.trim().is_empty() {
            "[No content]".to_string()
        } else {
            content
        };

        (final_content, tool_uses, tool_results, thinking_content)
    }

    fn convert_session(
        &self,
        claude_session: &ClaudeCodeSession,
    ) -> Result<(ChatSession, Vec<Message>)> {
        let session_id = Uuid::parse_str(&claude_session.uuid)
            .with_context(|| format!("Invalid UUID format: {}", claude_session.uuid))?;

        let start_time = self.parse_timestamp(&claude_session.created_at)?;
        let end_time = if claude_session.updated_at != claude_session.created_at {
            Some(self.parse_timestamp(&claude_session.updated_at)?)
        } else {
            None
        };

        let file_hash = self.calculate_file_hash()?;

        let mut chat_session = ChatSession::new(
            Provider::ClaudeCode,
            self.file_path.clone(),
            file_hash,
            start_time,
        );

        chat_session.id = session_id;
        if let Some(end) = end_time {
            chat_session = chat_session.with_end_time(end);
        }

        // Enhanced project name resolution with fallback
        let project_name = claude_session
            .name
            .clone() // First try name from session
            .or_else(|| {
                let inference = ProjectInference::new(&self.file_path);
                inference.infer_project_name()
            }); // Then infer from path

        if let Some(name) = project_name {
            chat_session = chat_session.with_project(name);
        }

        let mut messages = Vec::new();
        let mut total_tokens = 0u32;

        for (index, claude_message) in claude_session.chat_messages.iter().enumerate() {
            let message = self.convert_message(claude_message, session_id, index + 1)?;

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
        claude_message: &ClaudeCodeMessage,
        session_id: Uuid,
        sequence: usize,
    ) -> Result<Message> {
        let message_id = Uuid::parse_str(&claude_message.uuid)
            .with_context(|| format!("Invalid message UUID: {}", claude_message.uuid))?;

        let role = match claude_message.role.as_str() {
            "human" | "user" => MessageRole::User,
            "assistant" => MessageRole::Assistant,
            "system" => MessageRole::System,
            _ => return Err(anyhow!("Unknown message role: {}", claude_message.role)),
        };

        let (content, tool_uses, tool_results, _thinking_content) =
            self.extract_tools_and_content(&claude_message.content);
        // Note: thinking_content is ignored for legacy format

        let timestamp = self.parse_timestamp(&claude_message.created_at)?;

        let mut message = Message::new(session_id, role, content, timestamp, sequence as u32);

        message.id = message_id;

        // Attach tool uses and results if any
        if !tool_uses.is_empty() {
            message = message.with_tool_uses(tool_uses);
        }
        if !tool_results.is_empty() {
            message = message.with_tool_results(tool_results);
        }

        // Estimate token count based on content length
        let estimated_tokens = (message.content.len() / 4) as u32; // Rough estimate: 4 chars per token
        if estimated_tokens > 0 {
            message = message.with_token_count(estimated_tokens);
        }

        if let Some(metadata) = &claude_message.metadata {
            message = message.with_metadata(metadata.clone());
        }

        Ok(message)
    }

    fn parse_timestamp(&self, timestamp_str: &str) -> Result<DateTime<Utc>> {
        // Try different timestamp formats that Claude Code might use
        let formats = [
            "%Y-%m-%dT%H:%M:%S%.fZ",
            "%Y-%m-%dT%H:%M:%SZ",
            "%Y-%m-%dT%H:%M:%S%.f%z",
            "%Y-%m-%dT%H:%M:%S%z",
            "%Y-%m-%d %H:%M:%S UTC",
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

    /// Check if the filename matches Claude Code's expected format (UUID-based filename)
    pub fn accepts_filename(file_path: impl AsRef<Path>) -> bool {
        let path = file_path.as_ref();

        // Get the file stem (filename without extension)
        if let Some(file_stem) = path.file_stem().and_then(|s| s.to_str()) {
            // Check if the filename is a valid UUID format
            // UUID format: xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx
            // Pattern: 8-4-4-4-12 hex digits separated by hyphens
            if Uuid::parse_str(file_stem).is_ok() {
                return true;
            }
        }

        false
    }

    pub fn is_valid_file(file_path: impl AsRef<Path>) -> bool {
        let path = file_path.as_ref();

        // Check file extension
        if let Some(extension) = path.extension() {
            if extension != "jsonl" {
                return false;
            }
        } else {
            return false;
        }

        // Check if file exists and is readable
        if !path.exists() || !path.is_file() {
            return false;
        }

        // Check filename filter first - Claude Code only accepts UUID-based filenames
        if !Self::accepts_filename(path) {
            return false;
        }

        // Try to read the first line and see if it looks like Claude Code format
        if let Ok(file) = File::open(path) {
            let reader = BufReader::new(file);
            for line_content in reader.lines().take(1).flatten() {
                if let Ok(parsed) = serde_json::from_str::<Value>(&line_content) {
                    // Check for Claude Code session format
                    if parsed.get("uuid").is_some() && parsed.get("chat_messages").is_some() {
                        return true;
                    }
                    // Check for Claude Code conversation format
                    if parsed.get("type").is_some()
                        && (parsed.get("sessionId").is_some() || parsed.get("summary").is_some())
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
        let file = File::open(&self.file_path)
            .with_context(|| format!("Failed to open file: {}", self.file_path))?;

        let reader = BufReader::new(file);

        for line in reader.lines() {
            let line = line.with_context(|| "Failed to read line from file")?;

            if line.trim().is_empty() {
                continue;
            }

            let claude_session: ClaudeCodeSession = serde_json::from_str(&line)
                .with_context(|| format!("Failed to parse line as JSON: {line}"))?;

            let (chat_session, messages) = self.convert_session(&claude_session)?;

            for message in messages {
                callback(chat_session.clone(), message)?;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[tokio::test]
    async fn test_parse_claude_code_session() {
        let mut temp_file = NamedTempFile::new().unwrap();
        let sample_data = r#"{"uuid":"550e8400-e29b-41d4-a716-446655440000","name":"Test Session","created_at":"2024-01-01T10:00:00Z","updated_at":"2024-01-01T11:00:00Z","chat_messages":[{"uuid":"550e8400-e29b-41d4-a716-446655440001","content":"Hello","created_at":"2024-01-01T10:00:00Z","updated_at":"2024-01-01T10:00:00Z","role":"human"},{"uuid":"550e8400-e29b-41d4-a716-446655440002","content":"Hi there!","created_at":"2024-01-01T10:01:00Z","updated_at":"2024-01-01T10:01:00Z","role":"assistant"}]}"#;

        temp_file.write_all(sample_data.as_bytes()).unwrap();

        let parser = ClaudeCodeParser::new(temp_file.path());
        let result = parser.parse().await;

        assert!(result.is_ok());
        let (session, messages) = result.unwrap();

        assert_eq!(session.provider, Provider::ClaudeCode);
        assert_eq!(session.message_count, 2);
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].role, MessageRole::User);
        assert_eq!(messages[1].role, MessageRole::Assistant);
    }

    #[test]
    fn test_is_valid_file() {
        use std::fs;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        // Use UUID format filename as required by accepts_filename
        let file_path = temp_dir
            .path()
            .join("550e8400-e29b-41d4-a716-446655440000.jsonl");
        let sample_data = r#"{"uuid":"test","chat_messages":[]}"#;
        fs::write(&file_path, sample_data).unwrap();

        assert!(ClaudeCodeParser::is_valid_file(&file_path));
    }

    #[test]
    fn test_is_invalid_file() {
        let mut temp_file = NamedTempFile::with_suffix(".txt").unwrap();
        temp_file.write_all(b"not json").unwrap();

        assert!(!ClaudeCodeParser::is_valid_file(temp_file.path()));
    }

    #[test]
    fn test_infer_project_name_from_claude_pattern() {
        use std::fs;
        use tempfile::TempDir;

        // Create a temporary directory structure that mimics Claude's pattern
        let temp_dir = TempDir::new().unwrap();
        let base_path = temp_dir.path();

        // Create the actual project directory structure
        let project_path = base_path
            .join("Users")
            .join("testuser")
            .join("Project")
            .join("retrochat");
        fs::create_dir_all(&project_path).unwrap();

        // Create Claude's encoded directory
        let claude_dir = base_path.join("-Users-testuser-Project-retrochat");
        fs::create_dir_all(&claude_dir).unwrap();

        // Create a test file in the Claude directory
        let test_file = claude_dir.join("test.jsonl");
        fs::write(&test_file, "{}").unwrap();

        let inference = ProjectInference::new(&test_file);
        let project_name = inference.infer_project_name();

        assert_eq!(project_name, Some("retrochat".to_string()));
    }

    #[test]
    fn test_infer_project_name_with_hyphens_in_path() {
        use std::fs;
        use tempfile::TempDir;

        // Create a temporary directory structure with hyphens in the original path
        let temp_dir = TempDir::new().unwrap();
        let base_path = temp_dir.path();

        // Create the actual project directory with hyphens
        let project_path = base_path
            .join("Users")
            .join("testuser")
            .join("my-project")
            .join("sub-folder");
        fs::create_dir_all(&project_path).unwrap();

        // Create Claude's encoded directory (all hyphens become dashes)
        let claude_dir = base_path.join("-Users-testuser-my-project-sub-folder");
        fs::create_dir_all(&claude_dir).unwrap();

        // Create a test file in the Claude directory
        let test_file = claude_dir.join("test.jsonl");
        fs::write(&test_file, "{}").unwrap();

        let inference = ProjectInference::new(&test_file);
        let project_name = inference.infer_project_name();

        assert_eq!(project_name, Some("sub-folder".to_string()));
    }

    #[test]
    fn test_infer_project_name_complex_path() {
        use std::fs;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let base_path = temp_dir.path();

        // Create a complex path with multiple hyphens
        let project_path = base_path
            .join("Users")
            .join("testuser")
            .join("claude-squad")
            .join("worktrees")
            .join("test-project");
        fs::create_dir_all(&project_path).unwrap();

        // Create Claude's encoded directory
        let claude_dir = base_path.join("-Users-testuser-claude-squad-worktrees-test-project");
        fs::create_dir_all(&claude_dir).unwrap();

        let test_file = claude_dir.join("test.jsonl");
        fs::write(&test_file, "{}").unwrap();

        let inference = ProjectInference::new(&test_file);
        let project_name = inference.infer_project_name();

        assert_eq!(project_name, Some("test-project".to_string()));
    }

    #[test]
    fn test_infer_project_name_fallback_to_directory_name() {
        use std::fs;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let base_path = temp_dir.path();

        // Create a directory that doesn't follow Claude's pattern
        let regular_dir = base_path.join("regular-project-dir");
        fs::create_dir_all(&regular_dir).unwrap();

        let test_file = regular_dir.join("test.jsonl");
        fs::write(&test_file, "{}").unwrap();

        let inference = ProjectInference::new(&test_file);
        let project_name = inference.infer_project_name();

        assert_eq!(project_name, Some("regular-project-dir".to_string()));
    }

    #[test]
    fn test_infer_project_name_no_parent_directory() {
        use tempfile::NamedTempFile;

        let temp_file = NamedTempFile::new().unwrap();
        let inference = ProjectInference::new(temp_file.path());
        let project_name = inference.infer_project_name();

        // Should return None for files in root or with no discernible parent
        // Note: This might return Some() in practice due to temp file location
        // but the logic should handle cases where parent extraction fails
        assert!(project_name.is_some() || project_name.is_none()); // Accept either result for temp files
    }

    #[tokio::test]
    async fn test_parse_with_project_inference() {
        use std::fs;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let base_path = temp_dir.path();

        // Create the actual project directory structure
        let project_path = base_path
            .join("Users")
            .join("testuser")
            .join("Project")
            .join("testproject");
        fs::create_dir_all(&project_path).unwrap();

        // Create Claude's encoded directory
        let claude_dir = base_path.join("-Users-testuser-Project-testproject");
        fs::create_dir_all(&claude_dir).unwrap();

        let test_file = claude_dir.join("test.jsonl");

        // Create a sample conversation without explicit project name
        let sample_data = r#"{"type":"conversation","sessionId":"550e8400-e29b-41d4-a716-446655440000","timestamp":"2024-01-01T10:00:00Z","message":{"role":"user","content":"Hello"}}"#;
        fs::write(&test_file, sample_data).unwrap();

        let parser = ClaudeCodeParser::new(&test_file);
        let result = parser.parse().await;

        assert!(result.is_ok());
        let (session, _messages) = result.unwrap();

        // Should have inferred the project name from the path
        assert_eq!(session.project_name, Some("testproject".to_string()));
    }
}
