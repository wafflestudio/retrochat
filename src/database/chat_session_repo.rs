use anyhow::Result as AnyhowResult;
use chrono::{DateTime, NaiveDateTime, Utc};
use rusqlite::{params, Result};
use uuid::Uuid;

use super::connection::DatabaseManager;
use crate::models::chat_session::{ChatSession, LlmProvider, SessionState};

fn parse_datetime(datetime_str: &str) -> Result<DateTime<Utc>, chrono::ParseError> {
    // Try RFC3339 format first
    if let Ok(dt) = DateTime::parse_from_rfc3339(datetime_str) {
        return Ok(dt.with_timezone(&Utc));
    }

    // Try SQLite datetime format: "YYYY-MM-DD HH:MM:SS" or "YYYY-MM-DD HH:MM:SS+00:00"
    if let Ok(naive_dt) = NaiveDateTime::parse_from_str(datetime_str, "%Y-%m-%d %H:%M:%S") {
        return Ok(naive_dt.and_utc());
    }

    // Try with timezone offset
    if let Ok(dt) = DateTime::parse_from_str(datetime_str, "%Y-%m-%d %H:%M:%S%z") {
        return Ok(dt.with_timezone(&Utc));
    }

    // If all else fails, try RFC3339 again for better error message
    DateTime::parse_from_rfc3339(datetime_str).map(|dt| dt.with_timezone(&Utc))
}

pub struct ChatSessionRepository {
    db: DatabaseManager,
}

impl ChatSessionRepository {
    pub fn new(db: DatabaseManager) -> Self {
        Self { db }
    }

    pub fn create(&self, session: &ChatSession) -> AnyhowResult<()> {
        self.db.with_transaction(|conn| {
            conn.execute(
                "INSERT INTO chat_sessions (
                    id, provider, project_name, start_time, end_time,
                    message_count, token_count, file_path, file_hash,
                    created_at, updated_at, state
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
                params![
                    session.id.to_string(),
                    session.provider.to_string(),
                    session.project_name,
                    session.start_time.to_rfc3339(),
                    session.end_time.map(|t| t.to_rfc3339()),
                    session.message_count,
                    session.token_count,
                    session.file_path,
                    session.file_hash,
                    session.created_at.to_rfc3339(),
                    session.updated_at.to_rfc3339(),
                    session.state.to_string()
                ],
            )?;
            Ok(())
        })
    }

    pub fn get_by_id(&self, id: &Uuid) -> AnyhowResult<Option<ChatSession>> {
        self.db.with_connection(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, provider, project_name, start_time, end_time,
                        message_count, token_count, file_path, file_hash,
                        created_at, updated_at, state
                 FROM chat_sessions WHERE id = ?1",
            )?;

            let mut rows = stmt.query_map([id.to_string()], |row| self.row_to_session(row))?;

            match rows.next() {
                Some(session) => Ok(Some(session?)),
                None => Ok(None),
            }
        })
    }

    pub fn get_all(&self) -> AnyhowResult<Vec<ChatSession>> {
        self.db.with_connection(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, provider, project_name, start_time, end_time,
                        message_count, token_count, file_path, file_hash,
                        created_at, updated_at, state
                 FROM chat_sessions ORDER BY updated_at DESC",
            )?;

            let session_iter = stmt.query_map([], |row| self.row_to_session(row))?;

            let mut sessions = Vec::new();
            for session in session_iter {
                sessions.push(session?);
            }
            Ok(sessions)
        })
    }

    pub fn update(&self, session: &ChatSession) -> AnyhowResult<()> {
        self.db.with_transaction(|conn| {
            let rows_affected = conn.execute(
                "UPDATE chat_sessions SET
                    provider = ?2, project_name = ?3, start_time = ?4, end_time = ?5,
                    message_count = ?6, token_count = ?7, file_path = ?8, file_hash = ?9,
                    updated_at = ?10, state = ?11
                 WHERE id = ?1",
                params![
                    session.id.to_string(),
                    session.provider.to_string(),
                    session.project_name,
                    session.start_time.to_rfc3339(),
                    session.end_time.map(|t| t.to_rfc3339()),
                    session.message_count,
                    session.token_count,
                    session.file_path,
                    session.file_hash,
                    session.updated_at.to_rfc3339(),
                    session.state.to_string()
                ],
            )?;

            if rows_affected == 0 {
                return Err(rusqlite::Error::QueryReturnedNoRows);
            }
            Ok(())
        })
    }

    pub fn delete(&self, id: &Uuid) -> AnyhowResult<bool> {
        self.db.with_transaction(|conn| {
            let rows_affected =
                conn.execute("DELETE FROM chat_sessions WHERE id = ?1", [id.to_string()])?;
            Ok(rows_affected > 0)
        })
    }

    pub fn get_by_provider(&self, provider: &LlmProvider) -> AnyhowResult<Vec<ChatSession>> {
        self.db.with_connection(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, provider, project_name, start_time, end_time,
                        message_count, token_count, file_path, file_hash,
                        created_at, updated_at, state
                 FROM chat_sessions WHERE provider = ?1 ORDER BY updated_at DESC",
            )?;

            let session_iter =
                stmt.query_map([provider.to_string()], |row| self.row_to_session(row))?;

            let mut sessions = Vec::new();
            for session in session_iter {
                sessions.push(session?);
            }
            Ok(sessions)
        })
    }

    fn row_to_session(&self, row: &rusqlite::Row) -> Result<ChatSession> {
        let id_str: String = row.get(0)?;
        let provider_str: String = row.get(1)?;
        let project_name: Option<String> = row.get(2)?;
        let start_time_str: String = row.get(3)?;
        let end_time_str: Option<String> = row.get(4)?;
        let message_count: u32 = row.get(5)?;
        let token_count: Option<u32> = row.get(6)?;
        let file_path: String = row.get(7)?;
        let file_hash: String = row.get(8)?;
        let created_at_str: String = row.get(9)?;
        let updated_at_str: String = row.get(10)?;
        let state_str: String = row.get(11)?;

        let id = Uuid::parse_str(&id_str).map_err(|_| {
            rusqlite::Error::InvalidColumnType(0, id_str, rusqlite::types::Type::Text)
        })?;

        let provider = provider_str.parse::<LlmProvider>().map_err(|_| {
            rusqlite::Error::InvalidColumnType(1, provider_str, rusqlite::types::Type::Text)
        })?;

        let start_time = DateTime::parse_from_rfc3339(&start_time_str)
            .map_err(|_| {
                rusqlite::Error::InvalidColumnType(3, start_time_str, rusqlite::types::Type::Text)
            })?
            .with_timezone(&Utc);

        let end_time = if let Some(end_time_str) = end_time_str {
            Some(
                DateTime::parse_from_rfc3339(&end_time_str)
                    .map_err(|_| {
                        rusqlite::Error::InvalidColumnType(
                            4,
                            end_time_str,
                            rusqlite::types::Type::Text,
                        )
                    })?
                    .with_timezone(&Utc),
            )
        } else {
            None
        };

        let created_at = parse_datetime(&created_at_str)
            .map_err(|_| {
                rusqlite::Error::InvalidColumnType(9, created_at_str, rusqlite::types::Type::Text)
            })?
            .with_timezone(&Utc);

        let updated_at = parse_datetime(&updated_at_str)
            .map_err(|_| {
                rusqlite::Error::InvalidColumnType(10, updated_at_str, rusqlite::types::Type::Text)
            })?
            .with_timezone(&Utc);

        let state = state_str.parse::<SessionState>().map_err(|_| {
            rusqlite::Error::InvalidColumnType(11, state_str, rusqlite::types::Type::Text)
        })?;

        Ok(ChatSession {
            id,
            provider,
            project_name,
            start_time,
            end_time,
            message_count,
            token_count,
            file_path,
            file_hash,
            created_at,
            updated_at,
            state,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn test_create_and_get_session() {
        let db = DatabaseManager::open_in_memory().unwrap();
        let repo = ChatSessionRepository::new(db);

        let session = ChatSession::new(
            LlmProvider::ClaudeCode,
            "test.jsonl".to_string(),
            "hash123".to_string(),
            Utc::now(),
        );

        repo.create(&session).unwrap();

        let retrieved = repo.get_by_id(&session.id).unwrap().unwrap();
        assert_eq!(retrieved.id, session.id);
        assert_eq!(retrieved.file_path, session.file_path);
        assert_eq!(retrieved.provider, session.provider);
    }

    #[test]
    fn test_get_by_provider() {
        let db = DatabaseManager::open_in_memory().unwrap();
        let repo = ChatSessionRepository::new(db);

        let session1 = ChatSession::new(
            LlmProvider::ClaudeCode,
            "test1.jsonl".to_string(),
            "hash1".to_string(),
            Utc::now(),
        );

        let session2 = ChatSession::new(
            LlmProvider::Gemini,
            "test2.json".to_string(),
            "hash2".to_string(),
            Utc::now(),
        );

        repo.create(&session1).unwrap();
        repo.create(&session2).unwrap();

        let claude_sessions = repo.get_by_provider(&LlmProvider::ClaudeCode).unwrap();
        assert_eq!(claude_sessions.len(), 1);
        assert_eq!(claude_sessions[0].provider, LlmProvider::ClaudeCode);
    }
}
