use anyhow::{Context, Result as AnyhowResult};
use chrono::{DateTime, Utc};
use sqlx::{sqlite::SqliteRow, Pool, Row, Sqlite};
use std::collections::HashMap;
use uuid::Uuid;

use super::connection::DatabaseManager;
use crate::models::DetectedTurn;

pub struct DetectedTurnRepository {
    pool: Pool<Sqlite>,
}

impl DetectedTurnRepository {
    pub fn new(db: &DatabaseManager) -> Self {
        Self {
            pool: db.pool().clone(),
        }
    }

    pub async fn create(&self, turn: &DetectedTurn) -> AnyhowResult<()> {
        let tool_usage_json = serde_json::to_string(&turn.tool_usage).ok();
        let files_read_json = serde_json::to_string(&turn.files_read).ok();
        let files_written_json = serde_json::to_string(&turn.files_written).ok();
        let files_modified_json = serde_json::to_string(&turn.files_modified).ok();
        let commands_executed_json = serde_json::to_string(&turn.commands_executed).ok();

        sqlx::query(
            r#"
            INSERT INTO detected_turns (
                id, session_id, turn_number,
                start_sequence, end_sequence, user_message_id,
                message_count, user_message_count, assistant_message_count, system_message_count,
                simple_message_count, tool_request_count, tool_result_count, thinking_count, slash_command_count,
                total_token_count, user_token_count, assistant_token_count,
                tool_call_count, tool_success_count, tool_error_count, tool_usage,
                files_read, files_written, files_modified, unique_files_touched,
                total_lines_added, total_lines_removed, total_lines_changed,
                bash_command_count, bash_success_count, bash_error_count, commands_executed,
                user_message_preview, assistant_message_preview,
                started_at, ended_at, duration_seconds, created_at
            ) VALUES (
                ?, ?, ?,
                ?, ?, ?,
                ?, ?, ?, ?,
                ?, ?, ?, ?, ?,
                ?, ?, ?,
                ?, ?, ?, ?,
                ?, ?, ?, ?,
                ?, ?, ?,
                ?, ?, ?, ?,
                ?, ?,
                ?, ?, ?, ?
            )
            "#,
        )
        .bind(turn.id.to_string())
        .bind(turn.session_id.to_string())
        .bind(turn.turn_number)
        .bind(turn.start_sequence)
        .bind(turn.end_sequence)
        .bind(turn.user_message_id.map(|id| id.to_string()))
        .bind(turn.message_count)
        .bind(turn.user_message_count)
        .bind(turn.assistant_message_count)
        .bind(turn.system_message_count)
        .bind(turn.simple_message_count)
        .bind(turn.tool_request_count)
        .bind(turn.tool_result_count)
        .bind(turn.thinking_count)
        .bind(turn.slash_command_count)
        .bind(turn.total_token_count)
        .bind(turn.user_token_count)
        .bind(turn.assistant_token_count)
        .bind(turn.tool_call_count)
        .bind(turn.tool_success_count)
        .bind(turn.tool_error_count)
        .bind(tool_usage_json)
        .bind(files_read_json)
        .bind(files_written_json)
        .bind(files_modified_json)
        .bind(turn.unique_files_touched)
        .bind(turn.total_lines_added)
        .bind(turn.total_lines_removed)
        .bind(turn.total_lines_changed)
        .bind(turn.bash_command_count)
        .bind(turn.bash_success_count)
        .bind(turn.bash_error_count)
        .bind(commands_executed_json)
        .bind(&turn.user_message_preview)
        .bind(&turn.assistant_message_preview)
        .bind(turn.started_at.to_rfc3339())
        .bind(turn.ended_at.to_rfc3339())
        .bind(turn.duration_seconds)
        .bind(turn.created_at.to_rfc3339())
        .execute(&self.pool)
        .await
        .context("Failed to create detected turn")?;

        Ok(())
    }

    pub async fn bulk_create(&self, turns: &[DetectedTurn]) -> AnyhowResult<()> {
        if turns.is_empty() {
            return Ok(());
        }

        let mut tx = self.pool.begin().await?;

        for turn in turns {
            let tool_usage_json = serde_json::to_string(&turn.tool_usage).ok();
            let files_read_json = serde_json::to_string(&turn.files_read).ok();
            let files_written_json = serde_json::to_string(&turn.files_written).ok();
            let files_modified_json = serde_json::to_string(&turn.files_modified).ok();
            let commands_executed_json = serde_json::to_string(&turn.commands_executed).ok();

            sqlx::query(
                r#"
                INSERT INTO detected_turns (
                    id, session_id, turn_number,
                    start_sequence, end_sequence, user_message_id,
                    message_count, user_message_count, assistant_message_count, system_message_count,
                    simple_message_count, tool_request_count, tool_result_count, thinking_count, slash_command_count,
                    total_token_count, user_token_count, assistant_token_count,
                    tool_call_count, tool_success_count, tool_error_count, tool_usage,
                    files_read, files_written, files_modified, unique_files_touched,
                    total_lines_added, total_lines_removed, total_lines_changed,
                    bash_command_count, bash_success_count, bash_error_count, commands_executed,
                    user_message_preview, assistant_message_preview,
                    started_at, ended_at, duration_seconds, created_at
                ) VALUES (
                    ?, ?, ?,
                    ?, ?, ?,
                    ?, ?, ?, ?,
                    ?, ?, ?, ?, ?,
                    ?, ?, ?,
                    ?, ?, ?, ?,
                    ?, ?, ?, ?,
                    ?, ?, ?,
                    ?, ?, ?, ?,
                    ?, ?,
                    ?, ?, ?, ?
                )
                "#,
            )
            .bind(turn.id.to_string())
            .bind(turn.session_id.to_string())
            .bind(turn.turn_number)
            .bind(turn.start_sequence)
            .bind(turn.end_sequence)
            .bind(turn.user_message_id.map(|id| id.to_string()))
            .bind(turn.message_count)
            .bind(turn.user_message_count)
            .bind(turn.assistant_message_count)
            .bind(turn.system_message_count)
            .bind(turn.simple_message_count)
            .bind(turn.tool_request_count)
            .bind(turn.tool_result_count)
            .bind(turn.thinking_count)
            .bind(turn.slash_command_count)
            .bind(turn.total_token_count)
            .bind(turn.user_token_count)
            .bind(turn.assistant_token_count)
            .bind(turn.tool_call_count)
            .bind(turn.tool_success_count)
            .bind(turn.tool_error_count)
            .bind(tool_usage_json)
            .bind(files_read_json)
            .bind(files_written_json)
            .bind(files_modified_json)
            .bind(turn.unique_files_touched)
            .bind(turn.total_lines_added)
            .bind(turn.total_lines_removed)
            .bind(turn.total_lines_changed)
            .bind(turn.bash_command_count)
            .bind(turn.bash_success_count)
            .bind(turn.bash_error_count)
            .bind(commands_executed_json)
            .bind(&turn.user_message_preview)
            .bind(&turn.assistant_message_preview)
            .bind(turn.started_at.to_rfc3339())
            .bind(turn.ended_at.to_rfc3339())
            .bind(turn.duration_seconds)
            .bind(turn.created_at.to_rfc3339())
            .execute(&mut *tx)
            .await
            .context("Failed to create detected turn in bulk")?;
        }

        tx.commit().await?;
        Ok(())
    }

    pub async fn get_by_id(&self, id: &Uuid) -> AnyhowResult<Option<DetectedTurn>> {
        let row = sqlx::query(
            r#"
            SELECT * FROM detected_turns WHERE id = ?
            "#,
        )
        .bind(id.to_string())
        .fetch_optional(&self.pool)
        .await
        .context("Failed to fetch detected turn by ID")?;

        match row {
            Some(row) => {
                let turn = self.row_to_detected_turn(&row)?;
                Ok(Some(turn))
            }
            None => Ok(None),
        }
    }

    pub async fn get_by_session(&self, session_id: &Uuid) -> AnyhowResult<Vec<DetectedTurn>> {
        let rows = sqlx::query(
            r#"
            SELECT * FROM detected_turns
            WHERE session_id = ?
            ORDER BY turn_number ASC
            "#,
        )
        .bind(session_id.to_string())
        .fetch_all(&self.pool)
        .await
        .context("Failed to fetch detected turns by session")?;

        let mut turns = Vec::new();
        for row in rows {
            let turn = self.row_to_detected_turn(&row)?;
            turns.push(turn);
        }

        Ok(turns)
    }

    pub async fn get_by_session_and_turn_number(
        &self,
        session_id: &Uuid,
        turn_number: i32,
    ) -> AnyhowResult<Option<DetectedTurn>> {
        let row = sqlx::query(
            r#"
            SELECT * FROM detected_turns
            WHERE session_id = ? AND turn_number = ?
            "#,
        )
        .bind(session_id.to_string())
        .bind(turn_number)
        .fetch_optional(&self.pool)
        .await
        .context("Failed to fetch detected turn by session and turn number")?;

        match row {
            Some(row) => {
                let turn = self.row_to_detected_turn(&row)?;
                Ok(Some(turn))
            }
            None => Ok(None),
        }
    }

    pub async fn count_by_session(&self, session_id: &Uuid) -> AnyhowResult<i64> {
        let count: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*) FROM detected_turns WHERE session_id = ?
            "#,
        )
        .bind(session_id.to_string())
        .fetch_one(&self.pool)
        .await
        .context("Failed to count detected turns by session")?;

        Ok(count)
    }

    pub async fn delete_by_session(&self, session_id: &Uuid) -> AnyhowResult<u64> {
        let result = sqlx::query(
            r#"
            DELETE FROM detected_turns WHERE session_id = ?
            "#,
        )
        .bind(session_id.to_string())
        .execute(&self.pool)
        .await
        .context("Failed to delete detected turns by session")?;

        Ok(result.rows_affected())
    }

    pub async fn delete(&self, id: &Uuid) -> AnyhowResult<bool> {
        let result = sqlx::query(
            r#"
            DELETE FROM detected_turns WHERE id = ?
            "#,
        )
        .bind(id.to_string())
        .execute(&self.pool)
        .await
        .context("Failed to delete detected turn")?;

        Ok(result.rows_affected() > 0)
    }

    /// Get turns with most file changes
    pub async fn get_turns_with_most_changes(&self, limit: i64) -> AnyhowResult<Vec<DetectedTurn>> {
        let rows = sqlx::query(
            r#"
            SELECT * FROM detected_turns
            ORDER BY total_lines_changed DESC
            LIMIT ?
            "#,
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .context("Failed to fetch turns with most changes")?;

        let mut turns = Vec::new();
        for row in rows {
            let turn = self.row_to_detected_turn(&row)?;
            turns.push(turn);
        }

        Ok(turns)
    }

    /// Get turns with errors
    pub async fn get_turns_with_errors(
        &self,
        session_id: Option<&Uuid>,
    ) -> AnyhowResult<Vec<DetectedTurn>> {
        let rows = if let Some(session_id) = session_id {
            sqlx::query(
                r#"
                SELECT * FROM detected_turns
                WHERE session_id = ? AND (tool_error_count > 0 OR bash_error_count > 0)
                ORDER BY turn_number ASC
                "#,
            )
            .bind(session_id.to_string())
            .fetch_all(&self.pool)
            .await
            .context("Failed to fetch turns with errors for session")?
        } else {
            sqlx::query(
                r#"
                SELECT * FROM detected_turns
                WHERE tool_error_count > 0 OR bash_error_count > 0
                ORDER BY started_at DESC
                "#,
            )
            .fetch_all(&self.pool)
            .await
            .context("Failed to fetch turns with errors")?
        };

        let mut turns = Vec::new();
        for row in rows {
            let turn = self.row_to_detected_turn(&row)?;
            turns.push(turn);
        }

        Ok(turns)
    }

    /// Get sessions that don't have detected turns yet
    pub async fn get_sessions_without_turns(&self) -> AnyhowResult<Vec<Uuid>> {
        let rows = sqlx::query(
            r#"
            SELECT cs.id FROM chat_sessions cs
            LEFT JOIN detected_turns dt ON cs.id = dt.session_id
            WHERE dt.id IS NULL
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .context("Failed to fetch sessions without turns")?;

        let mut session_ids = Vec::new();
        for row in rows {
            let id_str: String = row.try_get("id")?;
            let id = Uuid::parse_str(&id_str).context("Invalid session ID format")?;
            session_ids.push(id);
        }

        Ok(session_ids)
    }

    fn row_to_detected_turn(&self, row: &SqliteRow) -> AnyhowResult<DetectedTurn> {
        let id_str: String = row.try_get("id")?;
        let session_id_str: String = row.try_get("session_id")?;
        let user_message_id_str: Option<String> = row.try_get("user_message_id")?;
        let started_at_str: String = row.try_get("started_at")?;
        let ended_at_str: String = row.try_get("ended_at")?;
        let created_at_str: String = row.try_get("created_at")?;

        let id = Uuid::parse_str(&id_str).context("Invalid turn ID format")?;
        let session_id = Uuid::parse_str(&session_id_str).context("Invalid session ID format")?;
        let user_message_id = user_message_id_str
            .filter(|s| !s.is_empty())
            .map(|s| Uuid::parse_str(&s))
            .transpose()
            .context("Invalid user message ID format")?;

        let started_at = DateTime::parse_from_rfc3339(&started_at_str)
            .context("Invalid started_at format")?
            .with_timezone(&Utc);
        let ended_at = DateTime::parse_from_rfc3339(&ended_at_str)
            .context("Invalid ended_at format")?
            .with_timezone(&Utc);
        let created_at = DateTime::parse_from_rfc3339(&created_at_str)
            .context("Invalid created_at format")?
            .with_timezone(&Utc);

        // Parse JSON fields
        let tool_usage_json: Option<String> = row.try_get("tool_usage").ok();
        let tool_usage: HashMap<String, i32> = tool_usage_json
            .and_then(|json| serde_json::from_str(&json).ok())
            .unwrap_or_default();

        let files_read_json: Option<String> = row.try_get("files_read").ok();
        let files_read: Vec<String> = files_read_json
            .and_then(|json| serde_json::from_str(&json).ok())
            .unwrap_or_default();

        let files_written_json: Option<String> = row.try_get("files_written").ok();
        let files_written: Vec<String> = files_written_json
            .and_then(|json| serde_json::from_str(&json).ok())
            .unwrap_or_default();

        let files_modified_json: Option<String> = row.try_get("files_modified").ok();
        let files_modified: Vec<String> = files_modified_json
            .and_then(|json| serde_json::from_str(&json).ok())
            .unwrap_or_default();

        let commands_executed_json: Option<String> = row.try_get("commands_executed").ok();
        let commands_executed: Vec<String> = commands_executed_json
            .and_then(|json| serde_json::from_str(&json).ok())
            .unwrap_or_default();

        Ok(DetectedTurn {
            id,
            session_id,
            turn_number: row.try_get("turn_number")?,
            start_sequence: row.try_get("start_sequence")?,
            end_sequence: row.try_get("end_sequence")?,
            user_message_id,
            message_count: row.try_get("message_count")?,
            user_message_count: row.try_get("user_message_count")?,
            assistant_message_count: row.try_get("assistant_message_count")?,
            system_message_count: row.try_get("system_message_count")?,
            simple_message_count: row.try_get("simple_message_count")?,
            tool_request_count: row.try_get("tool_request_count")?,
            tool_result_count: row.try_get("tool_result_count")?,
            thinking_count: row.try_get("thinking_count")?,
            slash_command_count: row.try_get("slash_command_count")?,
            total_token_count: row.try_get("total_token_count").ok(),
            user_token_count: row.try_get("user_token_count").ok(),
            assistant_token_count: row.try_get("assistant_token_count").ok(),
            tool_call_count: row.try_get("tool_call_count")?,
            tool_success_count: row.try_get("tool_success_count")?,
            tool_error_count: row.try_get("tool_error_count")?,
            tool_usage,
            files_read,
            files_written,
            files_modified,
            unique_files_touched: row.try_get("unique_files_touched").unwrap_or(0),
            total_lines_added: row.try_get("total_lines_added").unwrap_or(0),
            total_lines_removed: row.try_get("total_lines_removed").unwrap_or(0),
            total_lines_changed: row.try_get("total_lines_changed").unwrap_or(0),
            bash_command_count: row.try_get("bash_command_count").unwrap_or(0),
            bash_success_count: row.try_get("bash_success_count").unwrap_or(0),
            bash_error_count: row.try_get("bash_error_count").unwrap_or(0),
            commands_executed,
            user_message_preview: row.try_get("user_message_preview").ok(),
            assistant_message_preview: row.try_get("assistant_message_preview").ok(),
            started_at,
            ended_at,
            duration_seconds: row.try_get("duration_seconds").ok(),
            created_at,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::{ChatSessionRepository, DatabaseManager};
    use crate::models::{ChatSession, Provider, SessionState};

    async fn setup_db() -> (DatabaseManager, Uuid) {
        let db = DatabaseManager::open_in_memory().await.unwrap();
        let session_repo = ChatSessionRepository::new(&db);

        // Create a session
        let session_id = Uuid::new_v4();
        let mut session = ChatSession::new(
            Provider::ClaudeCode,
            "/test/file.jsonl".to_string(),
            "test_hash".to_string(),
            Utc::now(),
        );
        session.id = session_id;
        session.set_state(SessionState::Imported);
        session_repo.create(&session).await.unwrap();

        (db, session_id)
    }

    #[tokio::test]
    async fn test_create_and_get_detected_turn() {
        let (db, session_id) = setup_db().await;
        let repo = DetectedTurnRepository::new(&db);

        let turn = DetectedTurn::new(session_id, 1, Utc::now())
            .with_boundaries(0, 10)
            .with_message_counts(10, 2, 7, 1)
            .with_tool_metrics(5, 4, 1);

        repo.create(&turn).await.unwrap();

        let retrieved = repo.get_by_id(&turn.id).await.unwrap();
        assert!(retrieved.is_some());

        let retrieved = retrieved.unwrap();
        assert_eq!(retrieved.session_id, session_id);
        assert_eq!(retrieved.turn_number, 1);
        assert_eq!(retrieved.message_count, 10);
        assert_eq!(retrieved.tool_call_count, 5);
    }

    #[tokio::test]
    async fn test_get_by_session() {
        let (db, session_id) = setup_db().await;
        let repo = DetectedTurnRepository::new(&db);

        // Create multiple turns
        for i in 0..3 {
            let turn = DetectedTurn::new(session_id, i, Utc::now());
            repo.create(&turn).await.unwrap();
        }

        let turns = repo.get_by_session(&session_id).await.unwrap();
        assert_eq!(turns.len(), 3);
        assert_eq!(turns[0].turn_number, 0);
        assert_eq!(turns[1].turn_number, 1);
        assert_eq!(turns[2].turn_number, 2);
    }

    #[tokio::test]
    async fn test_bulk_create() {
        let (db, session_id) = setup_db().await;
        let repo = DetectedTurnRepository::new(&db);

        let turns: Vec<DetectedTurn> = (0..5)
            .map(|i| DetectedTurn::new(session_id, i, Utc::now()))
            .collect();

        repo.bulk_create(&turns).await.unwrap();

        let count = repo.count_by_session(&session_id).await.unwrap();
        assert_eq!(count, 5);
    }

    #[tokio::test]
    async fn test_delete_by_session() {
        let (db, session_id) = setup_db().await;
        let repo = DetectedTurnRepository::new(&db);

        // Create turns
        for i in 0..3 {
            let turn = DetectedTurn::new(session_id, i, Utc::now());
            repo.create(&turn).await.unwrap();
        }

        let deleted = repo.delete_by_session(&session_id).await.unwrap();
        assert_eq!(deleted, 3);

        let count = repo.count_by_session(&session_id).await.unwrap();
        assert_eq!(count, 0);
    }

    #[tokio::test]
    async fn test_json_fields_serialization() {
        let (db, session_id) = setup_db().await;
        let repo = DetectedTurnRepository::new(&db);

        let mut tool_usage = HashMap::new();
        tool_usage.insert("Read".to_string(), 5);
        tool_usage.insert("Write".to_string(), 3);

        let turn = DetectedTurn::new(session_id, 1, Utc::now())
            .with_tool_usage(tool_usage)
            .with_file_lists(
                vec!["a.rs".to_string(), "b.rs".to_string()],
                vec!["c.rs".to_string()],
                vec![],
            )
            .with_bash_metrics(2, 2, 0, vec!["cargo test".to_string()]);

        repo.create(&turn).await.unwrap();

        let retrieved = repo.get_by_id(&turn.id).await.unwrap().unwrap();
        assert_eq!(retrieved.tool_usage.get("Read"), Some(&5));
        assert_eq!(retrieved.tool_usage.get("Write"), Some(&3));
        assert_eq!(retrieved.files_read.len(), 2);
        assert_eq!(retrieved.files_written.len(), 1);
        assert_eq!(retrieved.commands_executed, vec!["cargo test".to_string()]);
    }
}
