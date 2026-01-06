//! Cursor Client (VSCode-based app) parser
//!
//! This parser handles chat data from the Cursor IDE application,
//! which stores data in SQLite databases within the workspaceStorage directory.
//!
//! Data locations by platform:
//! - macOS: ~/Library/Application Support/Cursor/User/workspaceStorage
//! - Windows: %APPDATA%/Cursor/User/workspaceStorage
//! - Linux: ~/.config/Cursor/User/workspaceStorage
//!
//! Global storage is located at: {workspaceStorage}/../globalStorage/state.vscdb

use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, TimeZone, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use uuid::Uuid;

use crate::models::{ChatSession, Message, MessageRole};
use crate::models::{Provider, SessionState};

/// Composer message type constants
const MESSAGE_TYPE_USER: i64 = 1;
const MESSAGE_TYPE_ASSISTANT: i64 = 2;

/// Cursor Client composer message structure (legacy format)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComposerMessage {
    #[serde(rename = "type")]
    pub message_type: i64, // 1 = user, 2 = assistant
    #[serde(rename = "bubbleId")]
    pub bubble_id: Option<String>,
    pub text: Option<String>,
    #[serde(rename = "richText")]
    pub rich_text: Option<String>,
    pub timestamp: Option<i64>,
}

/// Cursor Client composer chat structure (legacy format with allComposers)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComposerChat {
    #[serde(rename = "composerId")]
    pub composer_id: String,
    pub conversation: Option<Vec<ComposerMessage>>,
    pub text: Option<String>,
    #[serde(rename = "richText")]
    pub rich_text: Option<String>,
    pub status: Option<String>,
    pub name: Option<String>,
    #[serde(rename = "lastUpdatedAt")]
    pub last_updated_at: Option<i64>,
    #[serde(rename = "createdAt")]
    pub created_at: Option<i64>,
}

/// Cursor Client composer data container (legacy format)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComposerData {
    #[serde(rename = "allComposers")]
    pub all_composers: Option<Vec<ComposerChat>>,
    #[serde(rename = "selectedComposerId")]
    pub selected_composer_id: Option<String>,
}

/// New format: Single composer data (stored at composerData:{id})
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SingleComposerData {
    #[serde(rename = "_v")]
    pub version: Option<i64>,
    #[serde(rename = "composerId")]
    pub composer_id: String,
    pub text: Option<String>,
    #[serde(rename = "richText")]
    pub rich_text: Option<String>,
    pub name: Option<String>,
    /// Can be either string (ISO format) or integer (milliseconds timestamp)
    #[serde(rename = "createdAt")]
    pub created_at: Option<serde_json::Value>,
    /// Can be either string (ISO format) or integer (milliseconds timestamp)
    #[serde(rename = "lastUpdatedAt")]
    pub last_updated_at: Option<serde_json::Value>,
    #[serde(rename = "fullConversationHeadersOnly")]
    pub conversation_headers: Option<Vec<BubbleHeader>>,
}

/// Bubble header in conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BubbleHeader {
    #[serde(rename = "bubbleId")]
    pub bubble_id: String,
    #[serde(rename = "type")]
    pub bubble_type: i64, // 1 = user, 2 = assistant
}

/// Bubble data (stored at bubbleId:{composerId}:{bubbleId})
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BubbleData {
    #[serde(rename = "_v")]
    pub version: Option<i64>,
    #[serde(rename = "type")]
    pub bubble_type: i64, // 1 = user, 2 = assistant
    #[serde(rename = "bubbleId")]
    pub bubble_id: String,
    pub text: Option<String>,
    #[serde(rename = "richText")]
    pub rich_text: Option<String>,
    /// Can be either string (ISO format) or integer (milliseconds timestamp)
    #[serde(rename = "createdAt")]
    pub created_at: Option<serde_json::Value>,
}

/// Chat bubble structure for legacy chat format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatBubble {
    #[serde(rename = "type")]
    pub bubble_type: Option<String>, // "user" or "ai"
    pub text: Option<String>,
    #[serde(rename = "modelType")]
    pub model_type: Option<String>,
    pub timestamp: Option<i64>,
}

/// Chat tab structure for legacy chat format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatTab {
    pub id: Option<String>,
    pub title: Option<String>,
    pub timestamp: Option<String>,
    pub bubbles: Option<Vec<ChatBubble>>,
}

/// Chat data structure for legacy chat format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatData {
    pub tabs: Option<Vec<ChatTab>>,
}

/// Workspace info
#[derive(Debug, Clone)]
pub struct WorkspaceInfo {
    pub id: String,
    pub folder: Option<String>,
    pub db_path: PathBuf,
}

/// Storage mode for reading data
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum StorageMode {
    Global,
    Workspace,
    #[default]
    Both,
}

pub struct CursorClientParser {
    /// Path to the workspaceStorage directory or a specific state.vscdb file
    path: PathBuf,
    /// Storage mode
    storage_mode: StorageMode,
}

impl CursorClientParser {
    /// Create a new parser with the given path
    ///
    /// The path can be:
    /// - A specific state.vscdb file
    /// - A workspace directory containing state.vscdb
    /// - The workspaceStorage directory containing multiple workspaces
    pub fn new(path: impl AsRef<Path>) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
            storage_mode: StorageMode::default(),
        }
    }

    /// Create a new parser with a specific storage mode
    pub fn with_storage_mode(mut self, mode: StorageMode) -> Self {
        self.storage_mode = mode;
        self
    }

    /// Get the default workspace storage path for the current platform
    pub fn get_default_workspace_path() -> Option<PathBuf> {
        let home = dirs::home_dir()?;

        #[cfg(target_os = "macos")]
        {
            Some(home.join("Library/Application Support/Cursor/User/workspaceStorage"))
        }

        #[cfg(target_os = "windows")]
        {
            Some(home.join("AppData/Roaming/Cursor/User/workspaceStorage"))
        }

        #[cfg(target_os = "linux")]
        {
            // Check if running in remote/SSH environment
            if std::env::var("SSH_CONNECTION").is_ok()
                || std::env::var("SSH_CLIENT").is_ok()
                || std::env::var("SSH_TTY").is_ok()
            {
                Some(home.join(".cursor-server/data/User/workspaceStorage"))
            } else {
                Some(home.join(".config/Cursor/User/workspaceStorage"))
            }
        }

        #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
        {
            None
        }
    }

    /// Get the global storage database path
    fn get_global_db_path(&self) -> Option<PathBuf> {
        let workspace_storage = if self.path.ends_with("state.vscdb") {
            self.path.parent()?.parent()?
        } else if self.path.join("state.vscdb").exists() {
            self.path.parent()?
        } else {
            &self.path
        };

        let global_path = workspace_storage.join("../globalStorage/state.vscdb");
        if global_path.exists() {
            Some(global_path)
        } else {
            None
        }
    }

    /// Parse all available chat sessions
    pub async fn parse(&self) -> Result<Vec<(ChatSession, Vec<Message>)>> {
        let mut results = Vec::new();

        // Determine what to parse based on path type
        if self.path.ends_with("state.vscdb") {
            // Single database file
            let sessions = self.parse_single_db(&self.path).await?;
            results.extend(sessions);
        } else if self.path.join("state.vscdb").exists() {
            // Single workspace directory
            let db_path = self.path.join("state.vscdb");
            let sessions = self.parse_single_db(&db_path).await?;
            results.extend(sessions);
        } else {
            // workspaceStorage directory - parse all workspaces
            let should_read_global =
                self.storage_mode == StorageMode::Global || self.storage_mode == StorageMode::Both;
            let should_read_workspace = self.storage_mode == StorageMode::Workspace
                || self.storage_mode == StorageMode::Both;

            // Parse global storage
            if should_read_global {
                if let Some(global_db) = self.get_global_db_path() {
                    if let Ok(sessions) = self.parse_global_storage(&global_db).await {
                        results.extend(sessions);
                    }
                }
            }

            // Parse workspace storage
            if should_read_workspace {
                if let Ok(entries) = std::fs::read_dir(&self.path) {
                    for entry in entries.flatten() {
                        if entry.path().is_dir() {
                            let db_path = entry.path().join("state.vscdb");
                            if db_path.exists() {
                                if let Ok(sessions) =
                                    self.parse_workspace_storage(&db_path, &entry.path()).await
                                {
                                    results.extend(sessions);
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(results)
    }

    /// Parse a single database file (auto-detect format)
    async fn parse_single_db(&self, db_path: &Path) -> Result<Vec<(ChatSession, Vec<Message>)>> {
        let mut results = Vec::new();

        // Try global storage format first
        if let Ok(sessions) = self.parse_global_storage(db_path).await {
            if !sessions.is_empty() {
                return Ok(sessions);
            }
        }

        // Try workspace storage format
        if let Ok(sessions) = self
            .parse_workspace_storage(db_path, db_path.parent().unwrap())
            .await
        {
            results.extend(sessions);
        }

        Ok(results)
    }

    /// Parse global storage database (cursorDiskKV table)
    async fn parse_global_storage(
        &self,
        db_path: &Path,
    ) -> Result<Vec<(ChatSession, Vec<Message>)>> {
        let conn = rusqlite::Connection::open(db_path).with_context(|| {
            format!(
                "Failed to open global storage database: {}",
                db_path.display()
            )
        })?;

        let mut results = Vec::new();

        // Get all composerData entries
        let mut stmt = conn.prepare(
            "SELECT key, value FROM cursorDiskKV WHERE key LIKE 'composerData:%' AND LENGTH(value) > 10"
        )?;

        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;

        for row in rows.flatten() {
            let (key, value) = row;
            let _composer_id = key.strip_prefix("composerData:").unwrap_or(&key);

            // Try new format first (single composer per key)
            if let Ok(single_composer) = serde_json::from_str::<SingleComposerData>(&value) {
                if let Ok((session, messages)) =
                    self.convert_single_composer_to_session(&single_composer, &conn, db_path)
                {
                    if !messages.is_empty() {
                        results.push((session, messages));
                    }
                }
                continue;
            }

            // Try legacy format (allComposers array)
            if let Ok(composer_data) = serde_json::from_str::<ComposerData>(&value) {
                if let Some(composers) = composer_data.all_composers {
                    for composer in composers {
                        if let Ok((session, messages)) =
                            self.convert_composer_to_session(&composer, db_path)
                        {
                            if !messages.is_empty() {
                                results.push((session, messages));
                            }
                        }
                    }
                }
            }
        }

        Ok(results)
    }

    /// Parse bubble-based chats from cursorDiskKV (for future use)
    #[allow(dead_code)]
    fn parse_bubble_chats(
        &self,
        conn: &rusqlite::Connection,
        db_path: &Path,
    ) -> Result<Vec<(ChatSession, Vec<Message>)>> {
        let mut stmt =
            conn.prepare("SELECT key, value FROM cursorDiskKV WHERE key LIKE 'bubbleId:%'")?;

        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;

        // Group bubbles by chat ID
        let mut chat_map: HashMap<String, Vec<ChatBubble>> = HashMap::new();

        for row in rows.flatten() {
            let (key, value) = row;
            // key format: bubbleId:{chatId}:{bubbleId}
            let parts: Vec<&str> = key.split(':').collect();
            if parts.len() >= 2 {
                let chat_id = parts[1].to_string();
                if let Ok(bubble) = serde_json::from_str::<ChatBubble>(&value) {
                    chat_map.entry(chat_id).or_default().push(bubble);
                }
            }
        }

        let mut results = Vec::new();

        for (chat_id, mut bubbles) in chat_map {
            // Filter out invalid bubbles
            bubbles.retain(|b| b.text.is_some() || b.bubble_type.is_some());
            if bubbles.is_empty() {
                continue;
            }

            // Sort by timestamp
            bubbles.sort_by(|a, b| a.timestamp.unwrap_or(0).cmp(&b.timestamp.unwrap_or(0)));

            let session_id = Uuid::new_v4();
            let first_bubble = bubbles.first();
            let last_bubble = bubbles.last();

            let start_time = first_bubble
                .and_then(|b| b.timestamp)
                .map(|ts| self.timestamp_to_datetime(ts))
                .unwrap_or_else(Utc::now);

            let end_time = last_bubble
                .and_then(|b| b.timestamp)
                .map(|ts| self.timestamp_to_datetime(ts));

            let file_hash = self.calculate_file_hash(db_path)?;
            let title = first_bubble
                .and_then(|b| b.text.as_ref())
                .map(|t| t.lines().next().unwrap_or("").to_string())
                .unwrap_or_else(|| format!("Chat {}", &chat_id[..8.min(chat_id.len())]));

            let mut session = ChatSession::new(
                Provider::CursorClient,
                db_path.to_string_lossy().to_string(),
                file_hash,
                start_time,
            );
            session.id = Uuid::parse_str(&chat_id).unwrap_or(session_id);
            session = session.with_project(title);
            if let Some(end) = end_time {
                if end != start_time {
                    session = session.with_end_time(end);
                }
            }

            let mut messages = Vec::new();
            for (idx, bubble) in bubbles.iter().enumerate() {
                let role = match bubble.bubble_type.as_deref() {
                    Some("user") => MessageRole::User,
                    Some("ai") | Some("assistant") => MessageRole::Assistant,
                    _ => MessageRole::User,
                };

                let content = bubble.text.clone().unwrap_or_default();
                if content.is_empty() {
                    continue;
                }

                let timestamp = bubble
                    .timestamp
                    .map(|ts| self.timestamp_to_datetime(ts))
                    .unwrap_or(start_time);

                let message = Message::new(session.id, role, content, timestamp, (idx + 1) as u32);
                messages.push(message);
            }

            if !messages.is_empty() {
                session.message_count = messages.len() as u32;
                session.set_state(SessionState::Imported);
                results.push((session, messages));
            }
        }

        Ok(results)
    }

    /// Parse workspace storage database (ItemTable)
    async fn parse_workspace_storage(
        &self,
        db_path: &Path,
        workspace_dir: &Path,
    ) -> Result<Vec<(ChatSession, Vec<Message>)>> {
        let conn = rusqlite::Connection::open(db_path)
            .with_context(|| format!("Failed to open workspace database: {}", db_path.display()))?;

        let mut results = Vec::new();

        // Get workspace folder info
        let workspace_folder = self.get_workspace_folder(workspace_dir);

        // Try to get composer data
        let composer_result: Result<String, _> = conn.query_row(
            "SELECT value FROM ItemTable WHERE [key] = 'composer.composerData'",
            [],
            |row| row.get(0),
        );

        if let Ok(value) = composer_result {
            if let Ok(composer_data) = serde_json::from_str::<ComposerData>(&value) {
                if let Some(composers) = composer_data.all_composers {
                    for composer in composers {
                        if let Ok((mut session, messages)) =
                            self.convert_composer_to_session(&composer, db_path)
                        {
                            if let Some(ref folder) = workspace_folder {
                                session = session.with_project(folder.clone());
                            }
                            if !messages.is_empty() {
                                results.push((session, messages));
                            }
                        }
                    }
                }
            }
        }

        // Try to get chat data (legacy format)
        let chat_result: Result<String, _> = conn.query_row(
            "SELECT value FROM ItemTable WHERE [key] = 'workbench.panel.aichat.view.aichat.chatdata'",
            [],
            |row| row.get(0),
        );

        if let Ok(value) = chat_result {
            if let Ok(chat_data) = serde_json::from_str::<ChatData>(&value) {
                if let Some(tabs) = chat_data.tabs {
                    for tab in tabs {
                        if let Ok((mut session, messages)) =
                            self.convert_chat_tab_to_session(&tab, db_path)
                        {
                            if let Some(ref folder) = workspace_folder {
                                session = session.with_project(folder.clone());
                            }
                            if !messages.is_empty() {
                                results.push((session, messages));
                            }
                        }
                    }
                }
            }
        }

        Ok(results)
    }

    /// Get workspace folder name from workspace.json
    fn get_workspace_folder(&self, workspace_dir: &Path) -> Option<String> {
        let workspace_json = workspace_dir.join("workspace.json");
        if let Ok(content) = std::fs::read_to_string(&workspace_json) {
            if let Ok(data) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(folder) = data.get("folder").and_then(|f| f.as_str()) {
                    // Extract folder name from path
                    let folder_path = folder.replace("file://", "");
                    return folder_path
                        .split('/')
                        .next_back()
                        .or_else(|| folder_path.split('\\').next_back())
                        .map(|s| s.to_string());
                }
            }
        }
        None
    }

    /// Convert a SingleComposerData (new format) to ChatSession and Messages
    fn convert_single_composer_to_session(
        &self,
        composer: &SingleComposerData,
        conn: &rusqlite::Connection,
        db_path: &Path,
    ) -> Result<(ChatSession, Vec<Message>)> {
        let session_id = Uuid::parse_str(&composer.composer_id).unwrap_or_else(|_| Uuid::new_v4());

        let start_time = composer
            .created_at
            .as_ref()
            .and_then(|v| self.parse_timestamp_value(v))
            .unwrap_or_else(Utc::now);

        let end_time = composer
            .last_updated_at
            .as_ref()
            .and_then(|v| self.parse_timestamp_value(v));

        // Create unique file_path for each composer to avoid duplicate detection
        let unique_file_path = format!("{}#{}", db_path.to_string_lossy(), composer.composer_id);
        let file_hash = self.calculate_composer_hash(&composer.composer_id, db_path)?;

        let mut session = ChatSession::new(
            Provider::CursorClient,
            unique_file_path,
            file_hash,
            start_time,
        );
        session.id = session_id;

        // Try to get name or extract from first message
        if let Some(name) = &composer.name {
            if !name.is_empty() {
                session = session.with_project(name.clone());
            }
        }

        if let Some(end) = end_time {
            if end != start_time {
                session = session.with_end_time(end);
            }
        }

        let mut messages = Vec::new();

        // Fetch actual bubble data for each header
        if let Some(headers) = &composer.conversation_headers {
            for (idx, header) in headers.iter().enumerate() {
                // Query bubble data from DB
                let bubble_key = format!("bubbleId:{}:{}", composer.composer_id, header.bubble_id);
                let bubble_result: Result<String, _> = conn.query_row(
                    "SELECT value FROM cursorDiskKV WHERE key = ?",
                    [&bubble_key],
                    |row| row.get(0),
                );

                if let Ok(bubble_value) = bubble_result {
                    if let Ok(bubble) = serde_json::from_str::<BubbleData>(&bubble_value) {
                        let role = match bubble.bubble_type {
                            MESSAGE_TYPE_USER => MessageRole::User,
                            MESSAGE_TYPE_ASSISTANT => MessageRole::Assistant,
                            _ => MessageRole::User,
                        };

                        // Extract text: try text field first, then richText
                        let content = bubble
                            .text
                            .clone()
                            .filter(|t| !t.is_empty())
                            .or_else(|| {
                                bubble.rich_text.as_ref().and_then(|rt| {
                                    // First try to extract from Lexical JSON format
                                    if let Some(extracted) = self.extract_text_from_rich_text(rt) {
                                        return Some(extracted);
                                    }
                                    // If it's not JSON, use richText directly as plain text
                                    if !rt.is_empty() && !rt.starts_with('{') {
                                        return Some(rt.clone());
                                    }
                                    None
                                })
                            })
                            .unwrap_or_default();

                        if content.is_empty() {
                            continue;
                        }

                        let timestamp = bubble
                            .created_at
                            .as_ref()
                            .and_then(|v| self.parse_timestamp_value(v))
                            .unwrap_or(start_time);

                        let message =
                            Message::new(session_id, role, content, timestamp, (idx + 1) as u32);
                        messages.push(message);

                        // Set project name from first user message if not set
                        if session.project_name.is_none() && idx == 0 {
                            let first_line = messages[0].content.lines().next().unwrap_or("");
                            let title = if first_line.len() > 50 {
                                format!("{}...", &first_line[..47])
                            } else {
                                first_line.to_string()
                            };
                            if !title.is_empty() {
                                session = session.with_project(title);
                            }
                        }
                    }
                }
            }
        }

        session.message_count = messages.len() as u32;
        session.set_state(SessionState::Imported);

        Ok((session, messages))
    }

    /// Extract plain text from Lexical richText JSON
    fn extract_text_from_rich_text(&self, rich_text: &str) -> Option<String> {
        let parsed: serde_json::Value = serde_json::from_str(rich_text).ok()?;
        let mut texts = Vec::new();
        Self::extract_text_recursive(&parsed, &mut texts);
        let result = texts.join("");
        if result.is_empty() {
            None
        } else {
            Some(result)
        }
    }

    /// Recursively extract text from Lexical JSON structure
    fn extract_text_recursive(value: &serde_json::Value, texts: &mut Vec<String>) {
        match value {
            serde_json::Value::Object(obj) => {
                // If this is a text node, extract the text
                if obj.get("type").and_then(|t| t.as_str()) == Some("text") {
                    if let Some(text) = obj.get("text").and_then(|t| t.as_str()) {
                        texts.push(text.to_string());
                    }
                }
                // If this is a paragraph or linebreak, add newline
                if obj.get("type").and_then(|t| t.as_str()) == Some("paragraph")
                    && !texts.is_empty()
                    && !texts.last().map(|s| s.ends_with('\n')).unwrap_or(false)
                {
                    texts.push("\n".to_string());
                }
                if obj.get("type").and_then(|t| t.as_str()) == Some("linebreak") {
                    texts.push("\n".to_string());
                }
                // Recurse into children
                if let Some(children) = obj.get("children") {
                    Self::extract_text_recursive(children, texts);
                }
                // Also check root
                if let Some(root) = obj.get("root") {
                    Self::extract_text_recursive(root, texts);
                }
            }
            serde_json::Value::Array(arr) => {
                for item in arr {
                    Self::extract_text_recursive(item, texts);
                }
            }
            _ => {}
        }
    }

    /// Convert a ComposerChat (legacy format) to ChatSession and Messages
    fn convert_composer_to_session(
        &self,
        composer: &ComposerChat,
        db_path: &Path,
    ) -> Result<(ChatSession, Vec<Message>)> {
        let session_id = Uuid::parse_str(&composer.composer_id).unwrap_or_else(|_| Uuid::new_v4());

        let start_time = composer
            .created_at
            .map(|ts| self.timestamp_to_datetime(ts))
            .unwrap_or_else(Utc::now);

        let end_time = composer
            .last_updated_at
            .map(|ts| self.timestamp_to_datetime(ts));

        let file_hash = self.calculate_file_hash(db_path)?;

        let mut session = ChatSession::new(
            Provider::CursorClient,
            db_path.to_string_lossy().to_string(),
            file_hash,
            start_time,
        );
        session.id = session_id;

        if let Some(name) = &composer.name {
            if !name.is_empty() {
                session = session.with_project(name.clone());
            }
        }

        if let Some(end) = end_time {
            if end != start_time {
                session = session.with_end_time(end);
            }
        }

        let mut messages = Vec::new();

        if let Some(conversation) = &composer.conversation {
            for (idx, msg) in conversation.iter().enumerate() {
                let role = match msg.message_type {
                    MESSAGE_TYPE_USER => MessageRole::User,
                    MESSAGE_TYPE_ASSISTANT => MessageRole::Assistant,
                    _ => MessageRole::User,
                };

                // Prefer text, fallback to rich_text
                let content = msg
                    .text
                    .clone()
                    .or_else(|| msg.rich_text.clone())
                    .unwrap_or_default();

                if content.is_empty() {
                    continue;
                }

                let timestamp = msg
                    .timestamp
                    .map(|ts| self.timestamp_to_datetime(ts))
                    .unwrap_or(start_time);

                let message = Message::new(session_id, role, content, timestamp, (idx + 1) as u32);
                messages.push(message);
            }
        }

        session.message_count = messages.len() as u32;
        session.set_state(SessionState::Imported);

        Ok((session, messages))
    }

    /// Convert a ChatTab to ChatSession and Messages
    fn convert_chat_tab_to_session(
        &self,
        tab: &ChatTab,
        db_path: &Path,
    ) -> Result<(ChatSession, Vec<Message>)> {
        let tab_id = tab.id.clone().unwrap_or_else(|| Uuid::new_v4().to_string());
        let session_id = Uuid::parse_str(&tab_id).unwrap_or_else(|_| Uuid::new_v4());

        let start_time = tab
            .timestamp
            .as_ref()
            .and_then(|ts| self.parse_timestamp_str(ts).ok())
            .unwrap_or_else(Utc::now);

        let file_hash = self.calculate_file_hash(db_path)?;

        let mut session = ChatSession::new(
            Provider::CursorClient,
            db_path.to_string_lossy().to_string(),
            file_hash,
            start_time,
        );
        session.id = session_id;

        if let Some(title) = &tab.title {
            if !title.is_empty() {
                session = session.with_project(title.clone());
            }
        }

        let mut messages = Vec::new();
        let mut last_timestamp = start_time;

        if let Some(bubbles) = &tab.bubbles {
            for (idx, bubble) in bubbles.iter().enumerate() {
                let role = match bubble.bubble_type.as_deref() {
                    Some("user") => MessageRole::User,
                    Some("ai") | Some("assistant") => MessageRole::Assistant,
                    _ => MessageRole::User,
                };

                let content = bubble.text.clone().unwrap_or_default();
                if content.is_empty() {
                    continue;
                }

                let timestamp = bubble
                    .timestamp
                    .map(|ts| self.timestamp_to_datetime(ts))
                    .unwrap_or(last_timestamp);

                last_timestamp = timestamp;

                let message = Message::new(session_id, role, content, timestamp, (idx + 1) as u32);
                messages.push(message);
            }
        }

        if let Some(last_msg) = messages.last() {
            if last_msg.timestamp != start_time {
                session = session.with_end_time(last_msg.timestamp);
            }
        }

        session.message_count = messages.len() as u32;
        session.set_state(SessionState::Imported);

        Ok((session, messages))
    }

    /// Convert timestamp (milliseconds) to DateTime
    fn timestamp_to_datetime(&self, timestamp_ms: i64) -> DateTime<Utc> {
        let secs = timestamp_ms / 1000;
        let nsecs = ((timestamp_ms % 1000) * 1_000_000) as u32;
        Utc.timestamp_opt(secs, nsecs)
            .single()
            .unwrap_or_else(Utc::now)
    }

    /// Parse timestamp from serde_json::Value (can be string or integer)
    fn parse_timestamp_value(&self, value: &serde_json::Value) -> Option<DateTime<Utc>> {
        match value {
            serde_json::Value::Number(n) => {
                if let Some(ms) = n.as_i64() {
                    return Some(self.timestamp_to_datetime(ms));
                }
                if let Some(ms) = n.as_u64() {
                    return Some(self.timestamp_to_datetime(ms as i64));
                }
                None
            }
            serde_json::Value::String(s) => self.parse_timestamp_str(s).ok(),
            _ => None,
        }
    }

    /// Parse timestamp string to DateTime
    fn parse_timestamp_str(&self, timestamp_str: &str) -> Result<DateTime<Utc>> {
        // Try different formats
        let formats = [
            "%Y-%m-%dT%H:%M:%S%.fZ",
            "%Y-%m-%dT%H:%M:%SZ",
            "%Y-%m-%dT%H:%M:%S%.f%z",
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

        // Try parsing as milliseconds
        if let Ok(ms) = timestamp_str.parse::<i64>() {
            return Ok(self.timestamp_to_datetime(ms));
        }

        Err(anyhow!("Unable to parse timestamp: {timestamp_str}"))
    }

    /// Calculate a hash for the database file
    fn calculate_file_hash(&self, path: &Path) -> Result<String> {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let metadata = std::fs::metadata(path)
            .with_context(|| format!("Failed to get file metadata: {}", path.display()))?;

        let mut hasher = DefaultHasher::new();
        path.to_string_lossy().hash(&mut hasher);
        metadata.len().hash(&mut hasher);
        if let Ok(modified) = metadata.modified() {
            if let Ok(duration) = modified.duration_since(std::time::UNIX_EPOCH) {
                duration.as_secs().hash(&mut hasher);
            }
        }

        Ok(format!("{:x}", hasher.finish()))
    }

    /// Calculate a unique hash for a specific composer
    fn calculate_composer_hash(&self, composer_id: &str, db_path: &Path) -> Result<String> {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        db_path.to_string_lossy().hash(&mut hasher);
        composer_id.hash(&mut hasher);

        Ok(format!("{:x}", hasher.finish()))
    }

    /// Check if the given path is a valid Cursor Client data path
    pub fn is_valid_file(path: impl AsRef<Path>) -> bool {
        let path = path.as_ref();

        // Check if it's a state.vscdb file
        if path.file_name() == Some(std::ffi::OsStr::new("state.vscdb")) {
            // Verify it's in a Cursor storage directory structure
            if let Some(parent) = path.parent() {
                // Could be workspace storage or global storage
                if parent.file_name() == Some(std::ffi::OsStr::new("globalStorage")) {
                    return true;
                }

                // Check if it's in workspaceStorage
                if let Some(grandparent) = parent.parent() {
                    if grandparent.file_name() == Some(std::ffi::OsStr::new("workspaceStorage")) {
                        return true;
                    }
                }
            }

            // Try to open and check for expected tables
            if let Ok(conn) = rusqlite::Connection::open(path) {
                // Check for cursorDiskKV table (global storage)
                let has_cursor_kv: bool = conn
                    .query_row(
                        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='cursorDiskKV'",
                        [],
                        |row| row.get(0),
                    )
                    .unwrap_or(false);

                // Check for ItemTable (workspace storage)
                let has_item_table: bool = conn
                    .query_row(
                        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='ItemTable'",
                        [],
                        |row| row.get(0),
                    )
                    .unwrap_or(false);

                return has_cursor_kv || has_item_table;
            }
        }

        // Check if it's a directory containing state.vscdb
        if path.is_dir() {
            let db_path = path.join("state.vscdb");
            if db_path.exists() {
                return Self::is_valid_file(&db_path);
            }

            // Check if it's the workspaceStorage directory
            if path.file_name() == Some(std::ffi::OsStr::new("workspaceStorage")) {
                return true;
            }
        }

        false
    }

    /// Check if this parser accepts the given filename
    pub fn accepts_filename(_path: impl AsRef<Path>) -> bool {
        // CursorClientParser handles directories and state.vscdb files
        // The actual validation is done in is_valid_file
        true
    }

    /// Parse with streaming callback
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_is_valid_file_workspace_storage() {
        let temp_dir = TempDir::new().unwrap();
        let workspace_storage = temp_dir.path().join("workspaceStorage");
        let workspace_dir = workspace_storage.join("abc123");
        fs::create_dir_all(&workspace_dir).unwrap();

        let db_path = workspace_dir.join("state.vscdb");

        // Create a SQLite database with ItemTable
        let conn = rusqlite::Connection::open(&db_path).unwrap();
        conn.execute(
            "CREATE TABLE ItemTable (key TEXT PRIMARY KEY, value TEXT)",
            [],
        )
        .unwrap();
        drop(conn);

        assert!(CursorClientParser::is_valid_file(&db_path));
    }

    #[test]
    fn test_is_valid_file_global_storage() {
        let temp_dir = TempDir::new().unwrap();
        let global_storage = temp_dir.path().join("globalStorage");
        fs::create_dir_all(&global_storage).unwrap();

        let db_path = global_storage.join("state.vscdb");

        // Create a SQLite database with cursorDiskKV
        let conn = rusqlite::Connection::open(&db_path).unwrap();
        conn.execute(
            "CREATE TABLE cursorDiskKV (key TEXT PRIMARY KEY, value TEXT)",
            [],
        )
        .unwrap();
        drop(conn);

        assert!(CursorClientParser::is_valid_file(&db_path));
    }

    #[test]
    fn test_is_invalid_file() {
        let temp_dir = TempDir::new().unwrap();
        let invalid_file = temp_dir.path().join("not_state.db");
        fs::write(&invalid_file, "").unwrap();

        assert!(!CursorClientParser::is_valid_file(&invalid_file));
    }

    #[test]
    fn test_timestamp_conversion() {
        let parser = CursorClientParser::new("/tmp");
        let timestamp_ms: i64 = 1704067200000; // 2024-01-01 00:00:00 UTC

        let dt = parser.timestamp_to_datetime(timestamp_ms);
        assert_eq!(dt.timestamp(), 1704067200);
    }

    #[test]
    fn test_get_default_workspace_path() {
        let path = CursorClientParser::get_default_workspace_path();
        // Should return Some on supported platforms
        #[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
        assert!(path.is_some());
    }

    #[tokio::test]
    async fn test_parse_empty_workspace() {
        let temp_dir = TempDir::new().unwrap();
        let workspace_dir = temp_dir.path().join("workspace1");
        fs::create_dir_all(&workspace_dir).unwrap();

        let db_path = workspace_dir.join("state.vscdb");

        // Create empty database
        let conn = rusqlite::Connection::open(&db_path).unwrap();
        conn.execute(
            "CREATE TABLE ItemTable (key TEXT PRIMARY KEY, value TEXT)",
            [],
        )
        .unwrap();
        drop(conn);

        let parser = CursorClientParser::new(&db_path);
        let result = parser.parse().await;

        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_parse_composer_data() {
        let temp_dir = TempDir::new().unwrap();
        let workspace_storage = temp_dir.path().join("workspaceStorage");
        let workspace_dir = workspace_storage.join("workspace1");
        fs::create_dir_all(&workspace_dir).unwrap();

        let db_path = workspace_dir.join("state.vscdb");

        // Create database with composer data
        let conn = rusqlite::Connection::open(&db_path).unwrap();
        conn.execute(
            "CREATE TABLE ItemTable (key TEXT PRIMARY KEY, value TEXT)",
            [],
        )
        .unwrap();

        let composer_data = r#"{
            "allComposers": [{
                "composerId": "550e8400-e29b-41d4-a716-446655440000",
                "name": "Test Conversation",
                "createdAt": 1704067200000,
                "lastUpdatedAt": 1704070800000,
                "conversation": [
                    {"type": 1, "text": "Hello", "timestamp": 1704067200000},
                    {"type": 2, "text": "Hi there!", "timestamp": 1704067260000}
                ]
            }]
        }"#;

        conn.execute(
            "INSERT INTO ItemTable (key, value) VALUES ('composer.composerData', ?)",
            [composer_data],
        )
        .unwrap();
        drop(conn);

        let parser = CursorClientParser::new(&db_path);
        let result = parser.parse().await;

        assert!(result.is_ok());
        let sessions = result.unwrap();
        assert_eq!(sessions.len(), 1);

        let (session, messages) = &sessions[0];
        assert_eq!(session.provider, Provider::CursorClient);
        assert_eq!(session.message_count, 2);
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].role, MessageRole::User);
        assert_eq!(messages[1].role, MessageRole::Assistant);
    }
}
