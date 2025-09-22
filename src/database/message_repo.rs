use anyhow::Result as AnyhowResult;
use chrono::{DateTime, Utc};
use rusqlite::{params, OptionalExtension, Result};
use uuid::Uuid;

use super::connection::DatabaseManager;
use crate::models::message::{Message, MessageRole};

pub struct MessageRepository {
    db: DatabaseManager,
}

impl MessageRepository {
    pub fn new(db: DatabaseManager) -> Self {
        Self { db }
    }

    pub fn create(&self, message: &Message) -> AnyhowResult<()> {
        self.db.with_transaction(|conn| {
            // Insert into main messages table
            conn.execute(
                "INSERT INTO messages (
                    id, session_id, role, content, timestamp, token_count,
                    tool_calls, metadata, sequence_number
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    message.id.to_string(),
                    message.session_id.to_string(),
                    message.role.to_string(),
                    message.content,
                    message.timestamp.to_rfc3339(),
                    message.token_count,
                    message
                        .tool_calls
                        .as_ref()
                        .and_then(|tc| serde_json::to_string(tc).ok()),
                    "{}",
                    message.sequence_number
                ],
            )?;

            // FTS table is automatically updated by triggers

            Ok(())
        })
    }

    pub fn get_by_id(&self, id: &Uuid) -> AnyhowResult<Option<Message>> {
        self.db.with_connection(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, session_id, role, content, timestamp, token_count,
                        tool_calls, sequence_number
                 FROM messages WHERE id = ?1",
            )?;

            let mut rows = stmt.query_map([id.to_string()], |row| self.row_to_message(row))?;

            match rows.next() {
                Some(message) => Ok(Some(message?)),
                None => Ok(None),
            }
        })
    }

    pub fn get_by_session(&self, session_id: &Uuid) -> AnyhowResult<Vec<Message>> {
        self.db.with_connection(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, session_id, role, content, timestamp, token_count,
                        tool_calls, sequence_number
                 FROM messages
                 WHERE session_id = ?1
                 ORDER BY sequence_number ASC",
            )?;

            let message_iter =
                stmt.query_map([session_id.to_string()], |row| self.row_to_message(row))?;

            let mut messages = Vec::new();
            for message in message_iter {
                messages.push(message?);
            }
            Ok(messages)
        })
    }

    pub fn search_content(&self, query: &str) -> AnyhowResult<Vec<Message>> {
        self.db.with_connection(|conn| {
            let mut stmt = conn.prepare(
                "SELECT m.id, m.session_id, m.role, m.content, m.timestamp, m.token_count,
                        m.tool_calls, m.sequence_number
                 FROM messages_fts
                 JOIN messages m ON messages_fts.message_id = m.id
                 WHERE messages_fts MATCH ?1
                 LIMIT 100",
            )?;

            let message_iter = stmt.query_map([query], |row| self.row_to_message(row))?;

            let mut messages = Vec::new();
            for message in message_iter {
                messages.push(message?);
            }
            Ok(messages)
        })
    }

    pub fn search_content_with_filters(
        &self,
        query: &str,
        providers: Option<&[String]>,
        projects: Option<&[String]>,
        date_range: Option<&crate::services::query_service::DateRange>,
    ) -> AnyhowResult<Vec<Message>> {
        self.db.with_connection(|conn| {
            let mut sql = String::from(
                "SELECT m.id, m.session_id, m.role, m.content, m.timestamp, m.token_count,
                        m.tool_calls, m.sequence_number
                 FROM messages_fts
                 JOIN messages m ON messages_fts.message_id = m.id
                 JOIN chat_sessions cs ON m.session_id = cs.id
                 WHERE messages_fts MATCH ?1",
            );

            let mut params: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(query.to_string())];
            let mut param_count = 1;

            // Add provider filter
            if let Some(providers) = providers {
                if !providers.is_empty() {
                    param_count += 1;
                    let placeholders = providers.iter().map(|_| "?").collect::<Vec<_>>().join(",");
                    sql.push_str(&format!(" AND cs.provider IN ({placeholders})"));
                    for provider in providers {
                        params.push(Box::new(provider.clone()));
                    }
                }
            }

            // Add project filter
            if let Some(projects) = projects {
                if !projects.is_empty() {
                    param_count += 1;
                    let placeholders = projects.iter().map(|_| "?").collect::<Vec<_>>().join(",");
                    sql.push_str(&format!(" AND cs.project_name IN ({placeholders})"));
                    for project in projects {
                        params.push(Box::new(project.clone()));
                    }
                }
            }

            // Add date range filter
            if let Some(date_range) = date_range {
                param_count += 1;
                sql.push_str(&format!(" AND m.timestamp >= ?{param_count}"));
                params.push(Box::new(format!("{}T00:00:00Z", date_range.start_date)));

                param_count += 1;
                sql.push_str(&format!(" AND m.timestamp <= ?{param_count}"));
                params.push(Box::new(format!("{}T23:59:59Z", date_range.end_date)));
            }

            sql.push_str(" ORDER BY m.timestamp DESC LIMIT 1000");

            let mut stmt = conn.prepare(&sql)?;
            let message_iter = stmt
                .query_map(rusqlite::params_from_iter(params.iter()), |row| {
                    self.row_to_message(row)
                })?;

            let mut messages = Vec::new();
            for message in message_iter {
                messages.push(message?);
            }
            Ok(messages)
        })
    }

    pub fn delete(&self, id: &Uuid) -> AnyhowResult<bool> {
        self.db.with_transaction(|conn| {
            // FTS table is automatically updated by triggers
            let rows_affected =
                conn.execute("DELETE FROM messages WHERE id = ?1", [id.to_string()])?;

            Ok(rows_affected > 0)
        })
    }

    pub fn get_next_sequence_number(&self, session_id: &Uuid) -> AnyhowResult<u32> {
        self.db.with_connection(|conn| {
            let max_seq: Option<u32> = conn
                .query_row(
                    "SELECT MAX(sequence_number) FROM messages WHERE session_id = ?1",
                    [session_id.to_string()],
                    |row| row.get(0),
                )
                .optional()?;

            Ok(max_seq.unwrap_or(0) + 1)
        })
    }

    fn row_to_message(&self, row: &rusqlite::Row) -> Result<Message> {
        let id_str: String = row.get(0)?;
        let session_id_str: String = row.get(1)?;
        let role_str: String = row.get(2)?;
        let content: String = row.get(3)?;
        let timestamp_str: String = row.get(4)?;
        let token_count: Option<u32> = row.get(5)?;
        let tool_calls_str: Option<String> = row.get(6)?;
        let sequence_number: u32 = row.get(7)?;

        let id = Uuid::parse_str(&id_str).map_err(|_| {
            rusqlite::Error::InvalidColumnType(0, id_str, rusqlite::types::Type::Text)
        })?;

        let session_id = Uuid::parse_str(&session_id_str).map_err(|_| {
            rusqlite::Error::InvalidColumnType(1, session_id_str, rusqlite::types::Type::Text)
        })?;

        let role = role_str.parse::<MessageRole>().map_err(|_| {
            rusqlite::Error::InvalidColumnType(2, role_str, rusqlite::types::Type::Text)
        })?;

        let timestamp = DateTime::parse_from_rfc3339(&timestamp_str)
            .map_err(|_| {
                rusqlite::Error::InvalidColumnType(4, timestamp_str, rusqlite::types::Type::Text)
            })?
            .with_timezone(&Utc);

        let tool_calls = if let Some(tool_calls_str) = tool_calls_str {
            Some(serde_json::from_str(&tool_calls_str).map_err(|_| {
                rusqlite::Error::InvalidColumnType(6, tool_calls_str, rusqlite::types::Type::Text)
            })?)
        } else {
            None
        };

        Ok(Message {
            id,
            session_id,
            role,
            content,
            timestamp,
            token_count,
            tool_calls,
            metadata: None,
            sequence_number,
        })
    }

    pub fn count_all(&self) -> AnyhowResult<u64> {
        self.db.with_connection(|conn| {
            let count: u64 =
                conn.query_row("SELECT COUNT(*) FROM messages", [], |row| row.get(0))?;
            Ok(count)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::ChatSessionRepository;
    use crate::models::chat_session::{ChatSession, LlmProvider};
    use crate::models::message::MessageRole;

    fn create_test_session(db: &DatabaseManager) -> Uuid {
        let session_repo = ChatSessionRepository::new(db.clone());
        let session = ChatSession::new(
            LlmProvider::ClaudeCode,
            "test.jsonl".to_string(),
            "hash123".to_string(),
            Utc::now(),
        );
        let session_id = session.id;
        session_repo.create(&session).unwrap();
        session_id
    }

    #[test]
    fn test_create_and_get_message() {
        let db = DatabaseManager::open_in_memory().unwrap();
        let repo = MessageRepository::new(db.clone());

        let session_id = create_test_session(&db);
        let message = Message::new(
            session_id,
            MessageRole::User,
            "Hello, world!".to_string(),
            Utc::now(),
            1,
        );

        repo.create(&message).unwrap();

        let retrieved = repo.get_by_id(&message.id).unwrap().unwrap();
        assert_eq!(retrieved.id, message.id);
        assert_eq!(retrieved.content, message.content);
        assert_eq!(retrieved.role, message.role);
    }

    #[test]
    fn test_get_by_session() {
        let db = DatabaseManager::open_in_memory().unwrap();
        let repo = MessageRepository::new(db.clone());

        let session_id = create_test_session(&db);
        let message1 = Message::new(
            session_id,
            MessageRole::User,
            "First message".to_string(),
            Utc::now(),
            1,
        );

        let message2 = Message::new(
            session_id,
            MessageRole::Assistant,
            "Second message".to_string(),
            Utc::now(),
            2,
        );

        repo.create(&message1).unwrap();
        repo.create(&message2).unwrap();

        let messages = repo.get_by_session(&session_id).unwrap();
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].sequence_number, 1);
        assert_eq!(messages[1].sequence_number, 2);
    }

    #[test]
    fn test_search_content() {
        let db = DatabaseManager::open_in_memory().unwrap();
        let repo = MessageRepository::new(db.clone());

        let session_id = create_test_session(&db);
        let message = Message::new(
            session_id,
            MessageRole::User,
            "This is about machine learning".to_string(),
            Utc::now(),
            1,
        );

        repo.create(&message).unwrap();

        let results = repo.search_content("machine").unwrap();
        assert_eq!(results.len(), 1);
        assert!(results[0].content.contains("machine learning"));
    }
}
