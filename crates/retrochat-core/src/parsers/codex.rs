use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs::File;
use std::io::{BufRead, BufReader, Lines};
use std::path::Path;
use uuid::Uuid;

use crate::models::{ChatSession, Message, MessageRole};
use crate::models::{Provider, SessionState};

use super::project_inference::ProjectInference;

// ===== New Event-based Format Structures =====

#[derive(Debug, Serialize, Deserialize)]
pub struct CodexEvent {
    pub timestamp: String,
    #[serde(rename = "type")]
    pub event_type: String,
    pub payload: Value,
}

// ===== Legacy Format Structures (for old Codex files) =====

#[derive(Debug, Serialize, Deserialize)]
pub struct LegacySessionMeta {
    pub id: String,
    pub timestamp: String,
    pub instructions: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LegacyMessage {
    #[serde(rename = "type")]
    pub msg_type: String,
    pub id: Option<String>,
    pub role: String,
    pub content: Vec<LegacyContent>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LegacyContent {
    #[serde(rename = "type")]
    pub content_type: String,
    pub text: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SessionMetaPayload {
    pub id: String,
    pub timestamp: String,
    pub cwd: Option<String>,
    pub instructions: Option<String>,
    pub git: Option<CodexGitInfo>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CodexGitInfo {
    pub commit_hash: Option<String>,
    pub branch: Option<String>,
    pub repository_url: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EventMsgPayload {
    #[serde(rename = "type")]
    pub msg_type: String,
    pub message: Option<String>,
    pub info: Option<TokenInfo>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TokenInfo {
    pub total_token_usage: Option<TotalTokenUsage>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TotalTokenUsage {
    pub input_tokens: Option<u32>,
    pub cached_input_tokens: Option<u32>,
    pub output_tokens: Option<u32>,
    pub reasoning_output_tokens: Option<u32>,
    pub total_tokens: Option<u32>,
}

pub struct CodexParser {
    file_path: String,
}

impl CodexParser {
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
        let mut session_meta: Option<SessionMetaPayload> = None;
        let mut messages: Vec<(String, MessageRole, String)> = Vec::new(); // (timestamp, role, content)
        let mut legacy_messages: Vec<(MessageRole, String)> = Vec::new(); // For legacy messages without timestamps
        let mut total_tokens: Option<u32> = None;

        for line in lines {
            let line = line.with_context(|| "Failed to read line from file")?;

            if line.trim().is_empty() {
                continue;
            }

            // Try to parse as generic JSON first
            let json_value: Value = match serde_json::from_str(&line) {
                Ok(v) => v,
                Err(_) => continue, // Skip invalid JSON lines
            };

            // Check if this is a new event-based format
            if let Some(event_type) = json_value.get("type").and_then(|t| t.as_str()) {
                // Check if this is wrapped in an event structure (has timestamp at root)
                if json_value.get("timestamp").is_some() && json_value.get("payload").is_some() {
                    // New event-based format
                    if let Ok(event) = serde_json::from_value::<CodexEvent>(json_value.clone()) {
                        match event.event_type.as_str() {
                            "session_meta" => {
                                if let Ok(meta) =
                                    serde_json::from_value::<SessionMetaPayload>(event.payload)
                                {
                                    session_meta = Some(meta);
                                }
                            }
                            "event_msg" => {
                                if let Ok(msg) =
                                    serde_json::from_value::<EventMsgPayload>(event.payload)
                                {
                                    match msg.msg_type.as_str() {
                                        "user_message" => {
                                            if let Some(content) = msg.message {
                                                messages.push((
                                                    event.timestamp,
                                                    MessageRole::User,
                                                    content,
                                                ));
                                            }
                                        }
                                        "agent_message" => {
                                            if let Some(content) = msg.message {
                                                messages.push((
                                                    event.timestamp,
                                                    MessageRole::Assistant,
                                                    content,
                                                ));
                                            }
                                        }
                                        "token_count" => {
                                            if let Some(info) = msg.info {
                                                if let Some(usage) = info.total_token_usage {
                                                    if let Some(total) = usage.total_tokens {
                                                        total_tokens = Some(total);
                                                    }
                                                }
                                            }
                                        }
                                        _ => {}
                                    }
                                }
                            }
                            "response_item" => {
                                // Handle response_item events (newer format)
                                if let Ok(legacy_msg) =
                                    serde_json::from_value::<LegacyMessage>(event.payload.clone())
                                {
                                    let role = match legacy_msg.role.as_str() {
                                        "user" => MessageRole::User,
                                        "assistant" => MessageRole::Assistant,
                                        _ => continue,
                                    };

                                    // Extract text content from all parts
                                    let content = legacy_msg
                                        .content
                                        .iter()
                                        .filter_map(|c| c.text.as_ref())
                                        .cloned()
                                        .collect::<Vec<_>>()
                                        .join("\n");

                                    if !content.is_empty() {
                                        // Use the event timestamp (NOT current time!)
                                        messages.push((event.timestamp.clone(), role, content));
                                    }
                                }
                            }
                            _ => {} // Ignore other event types
                        }
                    }
                } else if event_type == "message" {
                    // Legacy message format (no timestamps on individual messages)
                    if let Ok(legacy_msg) = serde_json::from_value::<LegacyMessage>(json_value) {
                        let role = match legacy_msg.role.as_str() {
                            "user" => MessageRole::User,
                            "assistant" => MessageRole::Assistant,
                            _ => continue,
                        };

                        // Extract text content from all parts
                        let content = legacy_msg
                            .content
                            .iter()
                            .filter_map(|c| c.text.as_ref())
                            .cloned()
                            .collect::<Vec<_>>()
                            .join("\n");

                        if !content.is_empty() {
                            // Store legacy messages separately (will add timestamps later based on session start time)
                            legacy_messages.push((role, content));
                        }
                    }
                }
            } else if json_value.get("id").is_some() && json_value.get("timestamp").is_some() {
                // Legacy session metadata format (first line)
                if let Ok(legacy_meta) = serde_json::from_value::<LegacySessionMeta>(json_value) {
                    // Convert legacy metadata to new format
                    session_meta = Some(SessionMetaPayload {
                        id: legacy_meta.id,
                        timestamp: legacy_meta.timestamp,
                        cwd: None,
                        instructions: legacy_meta.instructions,
                        git: None,
                    });
                }
            }
            // Ignore other line types (record_type: state, etc.)
        }

        // If no session metadata found, return error
        let meta = session_meta.ok_or_else(|| anyhow!("No session_meta found in Codex file"))?;

        // Convert legacy messages to timestamped messages using session start time
        if !legacy_messages.is_empty() {
            let session_start = self.parse_timestamp(&meta.timestamp)?;
            for (idx, (role, content)) in legacy_messages.into_iter().enumerate() {
                // Add 1 second per message to maintain order
                let timestamp = session_start + chrono::Duration::seconds(idx as i64);
                messages.push((timestamp.to_rfc3339(), role, content));
            }
        }

        self.convert_session(&meta, messages, total_tokens)
    }

    fn convert_session(
        &self,
        meta: &SessionMetaPayload,
        messages: Vec<(String, MessageRole, String)>,
        parsed_total_tokens: Option<u32>,
    ) -> Result<(ChatSession, Vec<Message>)> {
        let session_id = Uuid::parse_str(&meta.id)
            .with_context(|| format!("Invalid UUID format: {}", meta.id))?;

        let start_time = self.parse_timestamp(&meta.timestamp)?;
        let file_hash = self.calculate_file_hash()?;

        let mut chat_session = ChatSession::new(
            Provider::Codex,
            self.file_path.clone(),
            file_hash,
            start_time,
        );

        chat_session.id = session_id;

        // Determine project name - prioritize cwd inference, fallback to git or path
        let project_name = self
            .infer_project_name_by_cwd(meta)
            .or_else(|| self.infer_project_name_by_git(meta))
            .or_else(|| {
                let inference = ProjectInference::new(&self.file_path);
                inference.infer_project_name()
            });

        if let Some(name) = project_name {
            chat_session = chat_session.with_project(name);
        }

        let mut converted_messages = Vec::new();
        let mut estimated_total_tokens = 0u32;

        for (index, (timestamp_str, role, content)) in messages.iter().enumerate() {
            let timestamp = self.parse_timestamp(timestamp_str)?;

            // Ensure content is never empty to satisfy database constraint
            let content = if content.trim().is_empty() {
                "[No content]".to_string()
            } else {
                content.clone()
            };

            // Skip messages with no meaningful content and no tools
            if content == "[No content]" {
                continue;
            }

            // Generate a deterministic UUID for the message
            let message_id = self.generate_uuid_from_string(&format!("{session_id}-msg-{index}"));

            let mut message = Message::new(
                session_id,
                role.clone(),
                content,
                timestamp,
                (index + 1) as u32,
            );
            message.id = message_id;

            // Estimate token count based on content length (for individual message tracking)
            let estimated_tokens = (message.content.len() / 4) as u32; // Rough estimate: 4 chars per token
            if estimated_tokens > 0 {
                message = message.with_token_count(estimated_tokens);
                estimated_total_tokens += estimated_tokens;
            }

            converted_messages.push(message);
        }

        chat_session.message_count = converted_messages.len() as u32;

        // Use parsed token count if available, otherwise use estimated
        if let Some(total) = parsed_total_tokens {
            chat_session = chat_session.with_token_count(total);
        } else if estimated_total_tokens > 0 {
            chat_session = chat_session.with_token_count(estimated_total_tokens);
        }

        // Calculate end time from last message timestamp
        if let Some(last_message) = converted_messages.last() {
            if last_message.timestamp != start_time {
                chat_session = chat_session.with_end_time(last_message.timestamp);
            }
        }

        chat_session.set_state(SessionState::Imported);

        Ok((chat_session, converted_messages))
    }

    fn parse_timestamp(&self, timestamp_str: &str) -> Result<DateTime<Utc>> {
        // Try different timestamp formats
        let formats = [
            "%Y-%m-%dT%H:%M:%S%.fZ",
            "%Y-%m-%dT%H:%M:%SZ",
            "%Y-%m-%dT%H:%M:%S%.f%z",
            "%Y-%m-%dT%H:%M:%S%z",
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

    fn infer_project_name_by_cwd(&self, meta: &SessionMetaPayload) -> Option<String> {
        // Extract project name from cwd path
        // e.g., "/Users/u1trafast/Workspace/retrochat" -> "retrochat"
        meta.cwd.as_ref().and_then(|cwd_path| {
            Path::new(cwd_path)
                .file_name()
                .and_then(|name| name.to_str())
                .map(|s| s.to_string())
        })
    }

    fn infer_project_name_by_git(&self, meta: &SessionMetaPayload) -> Option<String> {
        // Extract project name from git repository URL
        // e.g., "git@github.com:user/project.git" -> "project"
        meta.git.as_ref().and_then(|git| {
            git.repository_url.as_ref().and_then(|url| {
                url.rsplit('/')
                    .next()
                    .map(|s| s.trim_end_matches(".git").to_string())
            })
        })
    }

    /// Check if the filename should be accepted by Codex parser
    /// Codex doesn't have strict filename requirements, so this always returns true
    pub fn accepts_filename(_file_path: impl AsRef<Path>) -> bool {
        // Codex doesn't have specific filename format requirements
        // Accept all files that pass the content validation
        true
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

        // Note: Codex doesn't enforce filename filtering, so we skip accepts_filename check
        // If you want to add filename filtering for Codex in the future, uncomment this:
        // if !Self::accepts_filename(path) {
        //     return false;
        // }

        // Try to read the first line and see if it looks like Codex format
        if let Ok(file) = File::open(path) {
            let reader = BufReader::new(file);
            for line_content in reader.lines().take(1).flatten() {
                if let Ok(parsed) = serde_json::from_str::<Value>(&line_content) {
                    // Check for new event-based format with session_meta
                    if parsed.get("type").and_then(|t| t.as_str()) == Some("session_meta")
                        && parsed.get("payload").is_some()
                    {
                        return true;
                    }
                    // Check for legacy format (has id and timestamp but no type field)
                    // Old format: {"id":"...","timestamp":"...","instructions":null}
                    if parsed.get("id").is_some()
                        && parsed.get("timestamp").is_some()
                        && parsed.get("type").is_none()
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
        let (chat_session, messages) = self.parse().await?;

        for message in messages {
            callback(chat_session.clone(), message)?;
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
    async fn test_parse_codex_session_new_format() {
        let mut temp_file = NamedTempFile::with_suffix(".jsonl").unwrap();
        let sample_data = r#"{"timestamp":"2025-10-12T14:10:16.717Z","type":"session_meta","payload":{"id":"0199d8c1-ffeb-7b21-9ebe-f35fbbcf7a59","timestamp":"2025-10-12T14:10:16.683Z","cwd":"/Users/test/project","git":{"commit_hash":"abc123","branch":"main","repository_url":"git@github.com:user/test-project.git"}}}
{"timestamp":"2025-10-12T17:53:40.556Z","type":"event_msg","payload":{"type":"user_message","message":"hello"}}
{"timestamp":"2025-10-12T17:53:43.040Z","type":"event_msg","payload":{"type":"agent_message","message":"Hey there! What can I help you with today?"}}"#;

        temp_file.write_all(sample_data.as_bytes()).unwrap();

        let parser = CodexParser::new(temp_file.path());
        let result = parser.parse().await;

        assert!(result.is_ok());
        let (session, messages) = result.unwrap();

        assert_eq!(session.provider, Provider::Codex);
        assert_eq!(session.message_count, 2);
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].role, MessageRole::User);
        assert_eq!(messages[0].content, "hello");
        assert_eq!(messages[1].role, MessageRole::Assistant);
        assert_eq!(
            messages[1].content,
            "Hey there! What can I help you with today?"
        );
        assert_eq!(session.project_name, Some("project".to_string())); // Extracted from cwd
    }

    #[tokio::test]
    async fn test_parse_codex_session_with_token_count() {
        let mut temp_file = NamedTempFile::with_suffix(".jsonl").unwrap();
        let sample_data = r#"{"timestamp":"2025-10-12T14:10:16.717Z","type":"session_meta","payload":{"id":"0199d8c1-ffeb-7b21-9ebe-f35fbbcf7a59","timestamp":"2025-10-12T14:10:16.683Z","cwd":"/Users/test/project","git":{"commit_hash":"abc123","branch":"main","repository_url":"git@github.com:user/test-project.git"}}}
{"timestamp":"2025-10-12T17:53:40.556Z","type":"event_msg","payload":{"type":"user_message","message":"hello"}}
{"timestamp":"2025-10-12T17:53:43.040Z","type":"event_msg","payload":{"type":"agent_message","message":"Hey there! What can I help you with today?"}}
{"timestamp":"2025-10-12T17:59:44.860Z","type":"event_msg","payload":{"type":"token_count","info":{"total_token_usage":{"input_tokens":6062,"cached_input_tokens":3968,"output_tokens":92,"reasoning_output_tokens":64,"total_tokens":6154}}}}"#;

        temp_file.write_all(sample_data.as_bytes()).unwrap();

        let parser = CodexParser::new(temp_file.path());
        let result = parser.parse().await;

        assert!(result.is_ok());
        let (session, messages) = result.unwrap();

        assert_eq!(session.provider, Provider::Codex);
        assert_eq!(session.message_count, 2);
        assert_eq!(messages.len(), 2);
        assert_eq!(session.token_count, Some(6154)); // Token count should be parsed
        assert_eq!(session.project_name, Some("project".to_string())); // Extracted from cwd
    }

    #[test]
    fn test_is_valid_file_new_format() {
        let mut temp_file = NamedTempFile::with_suffix(".jsonl").unwrap();
        let sample_data = r#"{"timestamp":"2025-10-12T14:10:16.717Z","type":"session_meta","payload":{"id":"test","timestamp":"2024-01-01T10:00:00Z"}}"#;
        temp_file.write_all(sample_data.as_bytes()).unwrap();

        assert!(CodexParser::is_valid_file(temp_file.path()));
    }

    #[test]
    fn test_is_invalid_file() {
        let mut temp_file = NamedTempFile::with_suffix(".txt").unwrap();
        temp_file.write_all(b"not json").unwrap();

        assert!(!CodexParser::is_valid_file(temp_file.path()));
    }
}
