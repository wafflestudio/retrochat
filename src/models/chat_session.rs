use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::provider::Provider;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SessionState {
    Created,
    Imported,
    Analyticsd,
}

impl std::fmt::Display for SessionState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SessionState::Created => write!(f, "created"),
            SessionState::Imported => write!(f, "imported"),
            SessionState::Analyticsd => write!(f, "analyticsd"),
        }
    }
}

impl std::str::FromStr for SessionState {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "created" => Ok(SessionState::Created),
            "imported" => Ok(SessionState::Imported),
            "analyticsd" => Ok(SessionState::Analyticsd),
            _ => Err(format!("Unknown session state: {s}")),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatSession {
    pub id: Uuid,
    pub provider: Provider,
    pub project_name: Option<String>,
    pub start_time: DateTime<Utc>,
    pub end_time: Option<DateTime<Utc>>,
    pub message_count: u32,
    pub token_count: Option<u32>,
    pub file_path: String,
    pub file_hash: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub state: SessionState,
}

impl ChatSession {
    pub fn new(
        provider: Provider,
        file_path: String,
        file_hash: String,
        start_time: DateTime<Utc>,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            provider,
            project_name: None,
            start_time,
            end_time: None,
            message_count: 0,
            token_count: None,
            file_path,
            file_hash,
            created_at: now,
            updated_at: now,
            state: SessionState::Created,
        }
    }

    pub fn with_project(mut self, project_name: String) -> Self {
        self.project_name = Some(project_name);
        self
    }

    pub fn with_end_time(mut self, end_time: DateTime<Utc>) -> Self {
        self.end_time = Some(end_time);
        self
    }

    pub fn with_token_count(mut self, token_count: u32) -> Self {
        self.token_count = Some(token_count);
        self
    }

    pub fn update_message_count(&mut self, count: u32) {
        self.message_count = count;
        self.updated_at = Utc::now();
    }

    pub fn set_state(&mut self, state: SessionState) {
        self.state = state;
        self.updated_at = Utc::now();
    }

    pub fn duration(&self) -> Option<chrono::Duration> {
        self.end_time.map(|end| end - self.start_time)
    }

    pub fn is_valid(&self) -> bool {
        if let Some(end_time) = self.end_time {
            if end_time <= self.start_time {
                return false;
            }
        }

        if let Some(token_count) = self.token_count {
            if token_count == 0 && self.message_count > 0 {
                // This might be valid for some providers that don't track tokens
            }
        }

        !self.file_path.is_empty() && !self.file_hash.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_chat_session() {
        let provider = Provider::ClaudeCode;
        let file_path = "/path/to/chat.jsonl".to_string();
        let file_hash = "abc123".to_string();
        let start_time = Utc::now();

        let session = ChatSession::new(
            provider.clone(),
            file_path.clone(),
            file_hash.clone(),
            start_time,
        );

        assert_eq!(session.provider, provider);
        assert_eq!(session.file_path, file_path);
        assert_eq!(session.file_hash, file_hash);
        assert_eq!(session.start_time, start_time);
        assert_eq!(session.message_count, 0);
        assert_eq!(session.state, SessionState::Created);
        assert!(session.is_valid());
    }

    #[test]
    fn test_session_with_end_time_before_start_time() {
        let start_time = Utc::now();
        let end_time = start_time - chrono::Duration::hours(1);

        let session = ChatSession::new(
            Provider::ClaudeCode,
            "/path/to/chat.jsonl".to_string(),
            "abc123".to_string(),
            start_time,
        )
        .with_end_time(end_time);

        assert!(!session.is_valid());
    }

    #[test]
    fn test_provider_display() {
        assert_eq!(Provider::ClaudeCode.to_string(), "Claude Code");
        assert_eq!(Provider::GeminiCLI.to_string(), "Gemini CLI");
        assert_eq!(Provider::Other("custom".to_string()).to_string(), "custom");
    }
}
