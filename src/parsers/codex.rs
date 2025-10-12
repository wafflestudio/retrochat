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

#[derive(Debug, Serialize, Deserialize)]
pub struct CodexGitInfo {
    pub commit_hash: Option<String>,
    pub branch: Option<String>,
    pub repository_url: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CodexSessionHeader {
    pub id: String,
    pub timestamp: String,
    pub instructions: Option<String>,
    pub git: Option<CodexGitInfo>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CodexContentItem {
    #[serde(rename = "type")]
    pub content_type: String,
    pub text: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CodexMessage {
    #[serde(rename = "type")]
    pub message_type: String,
    pub role: String,
    pub content: Vec<CodexContentItem>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CodexStateRecord {
    pub record_type: String,
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
        let mut session_header: Option<CodexSessionHeader> = None;
        let mut codex_messages: Vec<CodexMessage> = Vec::new();

        for line in lines {
            let line = line.with_context(|| "Failed to read line from file")?;

            if line.trim().is_empty() {
                continue;
            }

            // Try to parse as session header first (has 'id' and 'timestamp' fields)
            if session_header.is_none() {
                if let Ok(header) = serde_json::from_str::<CodexSessionHeader>(&line) {
                    session_header = Some(header);
                    continue;
                }
            }

            // Try to parse as state record (can be ignored)
            if let Ok(state) = serde_json::from_str::<CodexStateRecord>(&line) {
                if state.record_type == "state" {
                    continue;
                }
            }

            // Try to parse as message
            if let Ok(message) = serde_json::from_str::<CodexMessage>(&line) {
                if message.message_type == "message" {
                    codex_messages.push(message);
                }
            }
        }

        // If no session header found, return error
        let header =
            session_header.ok_or_else(|| anyhow!("No session header found in Codex file"))?;

        self.convert_session(&header, codex_messages)
    }

    fn convert_session(
        &self,
        header: &CodexSessionHeader,
        codex_messages: Vec<CodexMessage>,
    ) -> Result<(ChatSession, Vec<Message>)> {
        let session_id = Uuid::parse_str(&header.id)
            .with_context(|| format!("Invalid UUID format: {}", header.id))?;

        let start_time = self.parse_timestamp(&header.timestamp)?;
        let file_hash = self.calculate_file_hash()?;

        let mut chat_session = ChatSession::new(
            Provider::Codex,
            self.file_path.clone(),
            file_hash,
            start_time,
        );

        chat_session.id = session_id;

        // Determine project name from git info or path inference
        let project_name = header
            .git
            .as_ref()
            .and_then(|git| {
                git.repository_url.as_ref().and_then(|url| {
                    // Extract project name from git URL
                    // e.g., "git@github.com:user/project.git" -> "project"
                    url.rsplit('/')
                        .next()
                        .map(|s| s.trim_end_matches(".git").to_string())
                })
            })
            .or_else(|| {
                let inference = ProjectInference::new(&self.file_path);
                inference.infer_project_name()
            });

        if let Some(name) = project_name {
            chat_session = chat_session.with_project(name);
        }

        let mut messages = Vec::new();
        let mut total_tokens = 0u32;

        for (index, codex_message) in codex_messages.iter().enumerate() {
            let message = self.convert_message(codex_message, session_id, index + 1)?;

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
        codex_message: &CodexMessage,
        session_id: Uuid,
        sequence: usize,
    ) -> Result<Message> {
        let role = match codex_message.role.as_str() {
            "user" => MessageRole::User,
            "assistant" => MessageRole::Assistant,
            "system" => MessageRole::System,
            _ => return Err(anyhow!("Unknown message role: {}", codex_message.role)),
        };

        // Extract content from content array
        let content = codex_message
            .content
            .iter()
            .filter_map(|item| item.text.clone())
            .collect::<Vec<_>>()
            .join("\n");

        // Ensure content is never empty to satisfy database constraint
        let content = if content.trim().is_empty() {
            "[No content]".to_string()
        } else {
            content
        };

        // Use the session start time for messages since Codex doesn't provide individual message timestamps
        // We could increment by sequence if needed for ordering
        let timestamp = Utc::now();

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

        // Try to read the first line and see if it looks like Codex format
        if let Ok(file) = File::open(path) {
            let reader = BufReader::new(file);
            for line_content in reader.lines().take(1).flatten() {
                if let Ok(parsed) = serde_json::from_str::<Value>(&line_content) {
                    // Check for Codex session header format
                    if parsed.get("id").is_some()
                        && parsed.get("timestamp").is_some()
                        && (parsed.get("git").is_some() || parsed.get("instructions").is_some())
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
    async fn test_parse_codex_session() {
        let mut temp_file = NamedTempFile::with_suffix(".jsonl").unwrap();
        let sample_data = r#"{"id":"550e8400-e29b-41d4-a716-446655440000","timestamp":"2024-01-01T10:00:00Z","instructions":null,"git":{"commit_hash":"abc123","branch":"main","repository_url":"git@github.com:user/test-project.git"}}
{"record_type":"state"}
{"type":"message","role":"user","content":[{"type":"input_text","text":"Hello"}]}
{"type":"message","role":"assistant","content":[{"type":"text","text":"Hi there!"}]}"#;

        temp_file.write_all(sample_data.as_bytes()).unwrap();

        let parser = CodexParser::new(temp_file.path());
        let result = parser.parse().await;

        assert!(result.is_ok());
        let (session, messages) = result.unwrap();

        assert_eq!(session.provider, Provider::Codex);
        assert_eq!(session.message_count, 2);
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].role, MessageRole::User);
        assert_eq!(messages[1].role, MessageRole::Assistant);
    }

    #[test]
    fn test_is_valid_file() {
        let mut temp_file = NamedTempFile::with_suffix(".jsonl").unwrap();
        let sample_data = r#"{"id":"test","timestamp":"2024-01-01T10:00:00Z","git":{}}"#;
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
