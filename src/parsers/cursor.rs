use anyhow::{anyhow, Context, Result};
use bytes::{Buf, Bytes};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use uuid::Uuid;

use crate::models::chat_session::{LlmProvider, SessionState};
use crate::models::{ChatSession, Message, MessageRole};

mod blob_decoder {
    use super::*;

    /// Decode a protobuf varint from bytes
    pub fn decode_varint(buf: &mut impl Buf) -> Result<u64> {
        let mut result = 0u64;
        let mut shift = 0;

        for _ in 0..10 {
            // Max 10 bytes for varint
            if !buf.has_remaining() {
                return Err(anyhow!("Unexpected end of varint"));
            }

            let byte = buf.get_u8();
            result |= ((byte & 0x7f) as u64) << shift;

            if byte & 0x80 == 0 {
                return Ok(result);
            }

            shift += 7;
        }

        Err(anyhow!("Varint too long"))
    }

    /// Parse a single protobuf field
    pub fn parse_field(buf: &mut impl Buf) -> Result<Option<(u32, FieldValue)>> {
        if !buf.has_remaining() {
            return Ok(None);
        }

        // Read field key
        let key = decode_varint(buf)?;
        let field_number = (key >> 3) as u32;
        let wire_type = (key & 0x07) as u8;

        let value = match wire_type {
            0 => {
                // Varint
                let val = decode_varint(buf)?;
                FieldValue::Varint(val)
            }
            2 => {
                // Length-delimited
                let length = decode_varint(buf)? as usize;
                if buf.remaining() < length {
                    return Err(anyhow!("Not enough bytes for length-delimited field"));
                }
                let mut data = vec![0u8; length];
                buf.copy_to_slice(&mut data);

                // Try to decode as UTF-8 string
                if let Ok(text) = String::from_utf8(data.clone()) {
                    FieldValue::String(text)
                } else {
                    FieldValue::Bytes(data)
                }
            }
            _ => {
                return Err(anyhow!("Unsupported wire type: {wire_type}"));
            }
        };

        Ok(Some((field_number, value)))
    }

    #[derive(Debug, Clone)]
    #[allow(dead_code)]
    pub enum FieldValue {
        Varint(u64),
        String(String),
        Bytes(Vec<u8>),
    }

    /// Parse all fields from a protobuf message
    pub fn parse_message(data: &[u8]) -> Result<HashMap<u32, Vec<FieldValue>>> {
        let mut buf = Bytes::copy_from_slice(data);
        let mut fields: HashMap<u32, Vec<FieldValue>> = HashMap::new();

        while buf.has_remaining() {
            match parse_field(&mut buf)? {
                Some((field_number, value)) => {
                    fields.entry(field_number).or_default().push(value);
                }
                None => break,
            }
        }

        Ok(fields)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CursorChatMetadata {
    #[serde(rename = "agentId")]
    pub agent_id: String,
    #[serde(rename = "latestRootBlobId")]
    pub latest_root_blob_id: String,
    pub name: String,
    pub mode: String,
    #[serde(rename = "createdAt")]
    pub created_at: u64,
    #[serde(rename = "lastUsedModel")]
    pub last_used_model: String,
}

pub struct CursorParser {
    db_path: String,
}

impl CursorParser {
    pub fn new(db_path: impl AsRef<Path>) -> Self {
        Self {
            db_path: db_path.as_ref().to_string_lossy().to_string(),
        }
    }

    pub async fn parse(&self) -> Result<(ChatSession, Vec<Message>)> {
        // Read metadata from the database
        let metadata = self.read_metadata()?;

        // Create session from metadata
        let session_id = Uuid::parse_str(&metadata.agent_id)
            .with_context(|| format!("Invalid agent UUID format: {}", metadata.agent_id))?;

        let start_time = self.timestamp_to_datetime(metadata.created_at)?;
        let file_hash = self.calculate_file_hash()?;

        let mut chat_session = ChatSession::new(
            LlmProvider::Cursor,
            self.db_path.clone(),
            file_hash,
            start_time,
        );

        chat_session.id = session_id;

        // Infer project name from database path
        let project_name = self.infer_project_name(&metadata);

        if let Some(name) = project_name {
            chat_session = chat_session.with_project(name);
        }

        // Parse blobs to extract messages
        let messages = self.read_blobs(session_id, start_time)?;

        chat_session.message_count = messages.len() as u32;
        chat_session.set_state(SessionState::Imported);

        Ok((chat_session, messages))
    }

    fn read_blobs(&self, session_id: Uuid, default_time: DateTime<Utc>) -> Result<Vec<Message>> {
        let conn = rusqlite::Connection::open(&self.db_path)
            .with_context(|| format!("Failed to open Cursor database: {}", self.db_path))?;

        let mut stmt = conn.prepare("SELECT id, data FROM blobs")?;
        let mut rows = stmt.query([])?;

        let mut messages = Vec::new();
        let mut message_index = 1;

        while let Some(row) = rows.next()? {
            let blob_id: String = row.get(0)?;
            let blob_data: Vec<u8> = row.get(1)?;

            // Parse the protobuf blob
            match blob_decoder::parse_message(&blob_data) {
                Ok(fields) => {
                    // Try to extract message content
                    if let Some(content) = self.extract_message_content(&fields) {
                        let role = self.infer_message_role(&content, &fields);

                        let message =
                            Message::new(session_id, role, content, default_time, message_index);
                        messages.push(message);
                        message_index += 1;
                    }
                }
                Err(e) => {
                    // Log error but continue processing other blobs
                    eprintln!("Failed to parse blob {blob_id}: {e}");
                }
            }
        }

        // If no messages were extracted, create a placeholder
        if messages.is_empty() {
            messages.push(Message::new(
                session_id,
                MessageRole::System,
                "Cursor chat session (no messages could be decoded)".to_string(),
                default_time,
                1,
            ));
        }

        Ok(messages)
    }

    fn extract_message_content(
        &self,
        fields: &HashMap<u32, Vec<blob_decoder::FieldValue>>,
    ) -> Option<String> {
        use blob_decoder::FieldValue;

        // Try field 1 for simple message content
        if let Some(values) = fields.get(&1) {
            for value in values {
                if let FieldValue::String(text) = value {
                    // Skip if it looks like a blob ID (32 bytes hex or UUID)
                    if text.len() != 32 && text.len() != 36 && !text.is_empty() {
                        return Some(text.clone());
                    }
                }
            }
        }

        // Try field 4 for JSON-encoded messages
        if let Some(values) = fields.get(&4) {
            for value in values {
                if let FieldValue::String(text) = value {
                    // Try to parse as JSON
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(text) {
                        if let Some(content) = json.get("content") {
                            if let Some(content_str) = content.as_str() {
                                return Some(content_str.to_string());
                            }
                            // Handle array of content blocks (Cursor format)
                            if let Some(content_array) = content.as_array() {
                                let mut full_content = String::new();

                                for item in content_array {
                                    let block_type = item.get("type").and_then(|t| t.as_str());

                                    match block_type {
                                        Some("text") => {
                                            // Extract text content
                                            if let Some(text) =
                                                item.get("text").and_then(|t| t.as_str())
                                            {
                                                if !full_content.is_empty() {
                                                    full_content.push_str("\n\n");
                                                }
                                                full_content.push_str(text);
                                            }
                                        }
                                        Some("tool-call") => {
                                            // Extract tool call information
                                            let tool_name = item
                                                .get("toolName")
                                                .and_then(|t| t.as_str())
                                                .unwrap_or("unknown_tool");

                                            if !full_content.is_empty() {
                                                full_content.push_str("\n\n");
                                            }
                                            full_content.push_str(&format!("[Tool: {tool_name}]"));

                                            // Extract tool arguments
                                            if let Some(args) = item.get("args") {
                                                if let Some(args_obj) = args.as_object() {
                                                    full_content.push('\n');
                                                    for (key, val) in args_obj {
                                                        match val {
                                                            serde_json::Value::String(s) => {
                                                                full_content.push_str(&format!(
                                                                    "  {key}: {s}\n"
                                                                ));
                                                            }
                                                            serde_json::Value::Array(arr) => {
                                                                if !arr.is_empty() {
                                                                    full_content.push_str(
                                                                        &format!(
                                                                            "  {key}: {arr:?}\n"
                                                                        ),
                                                                    );
                                                                }
                                                            }
                                                            _ => {
                                                                full_content.push_str(&format!(
                                                                    "  {key}: {val}\n"
                                                                ));
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                        _ => {
                                            // Unknown block type, try to extract as text
                                            if let Some(text) =
                                                item.get("text").and_then(|t| t.as_str())
                                            {
                                                if !full_content.is_empty() {
                                                    full_content.push_str("\n\n");
                                                }
                                                full_content.push_str(text);
                                            }
                                        }
                                    }
                                }

                                if !full_content.is_empty() {
                                    return Some(full_content);
                                }
                            }
                        }
                    }
                    return Some(text.clone());
                }
            }
        }

        None
    }

    fn infer_message_role(
        &self,
        content: &str,
        fields: &HashMap<u32, Vec<blob_decoder::FieldValue>>,
    ) -> MessageRole {
        use blob_decoder::FieldValue;

        // Try to extract role from JSON in field 4
        if let Some(values) = fields.get(&4) {
            for value in values {
                if let FieldValue::String(text) = value {
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(text) {
                        if let Some(role_str) = json.get("role").and_then(|r| r.as_str()) {
                            return match role_str {
                                "user" => MessageRole::User,
                                "assistant" => MessageRole::Assistant,
                                "system" => MessageRole::System,
                                _ => MessageRole::User,
                            };
                        }
                    }
                }
            }
        }

        // Default heuristic: short messages are likely user messages
        if content.len() < 200 {
            MessageRole::User
        } else {
            MessageRole::Assistant
        }
    }

    fn infer_project_name(&self, metadata: &CursorChatMetadata) -> Option<String> {
        use std::path::PathBuf;

        let path = PathBuf::from(&self.db_path);

        // Try to find a meaningful project directory name
        // Path structure: .../chats/{hash}/{uuid}/store.db
        // We want to look beyond the Cursor-specific directories

        if let Some(uuid_dir) = path.parent() {
            if let Some(hash_dir) = uuid_dir.parent() {
                if let Some(chats_dir) = hash_dir.parent() {
                    // chats_dir is .cursor/chats
                    if let Some(cursor_dir) = chats_dir.parent() {
                        // cursor_dir is .cursor
                        if let Some(project_dir) = cursor_dir.parent() {
                            // This is the actual project directory
                            if let Some(project_name) = project_dir.file_name() {
                                let name = project_name.to_string_lossy().to_string();
                                // Skip generic names like "Users", "home", etc.
                                if !name.starts_with('.')
                                    && name != "Users"
                                    && name != "home"
                                    && name.len() > 1
                                {
                                    return Some(name);
                                }
                            }
                        }
                    }
                }

                // Fallback: Use metadata name if it's meaningful
                if !metadata.name.is_empty() && metadata.name.len() > 3 {
                    return Some(metadata.name.clone());
                }

                // Last resort: Use first 8 characters of hash directory
                if let Some(hash_name) = hash_dir.file_name() {
                    let hash_str = hash_name.to_string_lossy();
                    if hash_str.len() >= 8 {
                        return Some(format!("cursor-{}", &hash_str[..8]));
                    }
                }
            }
        }

        None
    }

    fn read_metadata(&self) -> Result<CursorChatMetadata> {
        let conn = rusqlite::Connection::open(&self.db_path)
            .with_context(|| format!("Failed to open Cursor database: {}", self.db_path))?;

        let mut stmt = conn.prepare("SELECT value FROM meta WHERE key = '0'")?;

        let hex_value: String = stmt
            .query_row([], |row| row.get::<_, String>(0))
            .context("Failed to read metadata from Cursor database")?;

        // Decode hex string to bytes
        let json_bytes = hex::decode(&hex_value).context("Failed to decode hex metadata")?;

        // Parse JSON
        let metadata: CursorChatMetadata =
            serde_json::from_slice(&json_bytes).context("Failed to parse metadata JSON")?;

        Ok(metadata)
    }

    fn timestamp_to_datetime(&self, timestamp_ms: u64) -> Result<DateTime<Utc>> {
        let timestamp_secs = (timestamp_ms / 1000) as i64;
        let dt = DateTime::from_timestamp(timestamp_secs, 0)
            .ok_or_else(|| anyhow!("Invalid timestamp: {timestamp_ms}"))?;
        Ok(dt)
    }

    fn calculate_file_hash(&self) -> Result<String> {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let metadata = std::fs::metadata(&self.db_path)
            .with_context(|| format!("Failed to get file metadata: {}", self.db_path))?;

        let mut hasher = DefaultHasher::new();
        self.db_path.hash(&mut hasher);
        metadata.len().hash(&mut hasher);
        if let Ok(modified) = metadata.modified() {
            if let Ok(duration) = modified.duration_since(std::time::UNIX_EPOCH) {
                duration.as_secs().hash(&mut hasher);
            }
        }

        Ok(format!("{:x}", hasher.finish()))
    }

    pub fn is_valid_file(file_path: impl AsRef<Path>) -> bool {
        let path = file_path.as_ref();

        // Check if it's a Cursor store.db file
        if path.file_name() != Some(std::ffi::OsStr::new("store.db")) {
            return false;
        }

        // Check if it's in a Cursor directory structure
        if let Some(parent) = path.parent() {
            if let Some(uuid_dir) = parent.file_name().and_then(|n| n.to_str()) {
                // Should be a UUID-like directory
                if uuid_dir.len() == 36 && uuid_dir.chars().filter(|&c| c == '-').count() == 4 {
                    if let Some(grandparent) = parent.parent() {
                        if let Some(hash_dir) = grandparent.file_name().and_then(|n| n.to_str()) {
                            // Should be a hash directory under chats
                            if hash_dir.len() == 32
                                && hash_dir.chars().all(|c| c.is_ascii_hexdigit())
                            {
                                if let Some(chats_dir) = grandparent.parent() {
                                    return chats_dir.file_name()
                                        == Some(std::ffi::OsStr::new("chats"));
                                }
                            }
                        }
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
        let (session, messages) = self.parse().await?;

        for message in messages {
            callback(session.clone(), message)?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_is_valid_file() {
        let temp_dir = TempDir::new().unwrap();
        let base_path = temp_dir.path();

        // Create Cursor directory structure
        let chats_dir = base_path.join("chats");
        let hash_dir = chats_dir.join("53460df9022de1a66445a5b78b067dd9");
        let uuid_dir = hash_dir.join("557abc41-6f00-41e7-bf7b-696c80d4ee94");
        fs::create_dir_all(&uuid_dir).unwrap();

        let store_db = uuid_dir.join("store.db");
        fs::write(&store_db, "").unwrap();

        assert!(CursorParser::is_valid_file(&store_db));
    }

    #[test]
    fn test_is_invalid_file() {
        let temp_dir = TempDir::new().unwrap();
        let invalid_file = temp_dir.path().join("not_store.db");
        fs::write(&invalid_file, "").unwrap();

        assert!(!CursorParser::is_valid_file(&invalid_file));
    }
}
