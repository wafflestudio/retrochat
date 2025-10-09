use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::Path;
use uuid::Uuid;

use crate::models::chat_session::{LlmProvider, SessionState};
use crate::models::{ChatSession, Message, MessageRole};

use super::project_inference::ProjectInference;

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
        let project_name = {
            let inference = ProjectInference::new(&self.db_path);
            inference.infer_project_name()
        };

        if let Some(name) = project_name {
            chat_session = chat_session.with_project(name);
        }

        // For now, we'll create a placeholder message since the blob format is binary
        // In the future, this could be enhanced to decode the binary format
        let placeholder_message = Message::new(
            session_id,
            MessageRole::System,
            format!(
                "Cursor chat session: {} (Binary data not yet decoded)",
                metadata.name
            ),
            start_time,
            1,
        );

        chat_session.message_count = 1;
        chat_session.set_state(SessionState::Imported);

        Ok((chat_session, vec![placeholder_message]))
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
