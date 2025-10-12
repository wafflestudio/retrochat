use anyhow::{Context, Result as AnyhowResult};
use chrono::{DateTime, NaiveDateTime, Utc};
use sqlx::{sqlite::SqliteRow, Pool, Row, Sqlite};
use uuid::Uuid;

use super::connection::DatabaseManager;
use crate::models::{ChatSession, LlmProvider, SessionState};

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
    pool: Pool<Sqlite>,
}

impl ChatSessionRepository {
    pub fn new(db: &DatabaseManager) -> Self {
        Self {
            pool: db.pool().clone(),
        }
    }

    pub async fn create(&self, session: &ChatSession) -> AnyhowResult<()> {
        sqlx::query(
            r#"
            INSERT INTO chat_sessions (
                id, provider, project_name, start_time, end_time,
                message_count, token_count, file_path, file_hash,
                created_at, updated_at, state
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(session.id.to_string())
        .bind(session.provider.to_string())
        .bind(session.project_name.as_ref())
        .bind(session.start_time.to_rfc3339())
        .bind(session.end_time.map(|t| t.to_rfc3339()))
        .bind(session.message_count)
        .bind(session.token_count)
        .bind(&session.file_path)
        .bind(&session.file_hash)
        .bind(session.created_at.to_rfc3339())
        .bind(session.updated_at.to_rfc3339())
        .bind(session.state.to_string())
        .execute(&self.pool)
        .await
        .context("Failed to create chat session")?;

        Ok(())
    }

    pub async fn get_by_id(&self, id: &Uuid) -> AnyhowResult<Option<ChatSession>> {
        let row = sqlx::query(
            r#"
            SELECT id, provider, project_name, start_time, end_time,
                   message_count, token_count, file_path, file_hash,
                   created_at, updated_at, state
            FROM chat_sessions WHERE id = ?
            "#,
        )
        .bind(id.to_string())
        .fetch_optional(&self.pool)
        .await
        .context("Failed to fetch chat session by ID")?;

        match row {
            Some(row) => {
                let session = self.row_to_session(&row)?;
                Ok(Some(session))
            }
            None => Ok(None),
        }
    }

    pub async fn get_all(&self) -> AnyhowResult<Vec<ChatSession>> {
        let rows = sqlx::query(
            r#"
            SELECT id, provider, project_name, start_time, end_time,
                   message_count, token_count, file_path, file_hash,
                   created_at, updated_at, state
            FROM chat_sessions ORDER BY updated_at DESC
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .context("Failed to fetch all chat sessions")?;

        let mut sessions = Vec::new();
        for row in rows {
            let session = self.row_to_session(&row)?;
            sessions.push(session);
        }

        Ok(sessions)
    }

    pub async fn update(&self, session: &ChatSession) -> AnyhowResult<()> {
        let result = sqlx::query(
            r#"
            UPDATE chat_sessions SET
                provider = ?, project_name = ?, start_time = ?, end_time = ?,
                message_count = ?, token_count = ?, file_path = ?, file_hash = ?,
                updated_at = ?, state = ?
            WHERE id = ?
            "#,
        )
        .bind(session.provider.to_string())
        .bind(session.project_name.as_ref())
        .bind(session.start_time.to_rfc3339())
        .bind(session.end_time.map(|t| t.to_rfc3339()))
        .bind(session.message_count)
        .bind(session.token_count)
        .bind(&session.file_path)
        .bind(&session.file_hash)
        .bind(session.updated_at.to_rfc3339())
        .bind(session.state.to_string())
        .bind(session.id.to_string())
        .execute(&self.pool)
        .await
        .context("Failed to update chat session")?;

        if result.rows_affected() == 0 {
            return Err(anyhow::anyhow!("Chat session not found"));
        }

        Ok(())
    }

    pub async fn delete(&self, id: &Uuid) -> AnyhowResult<bool> {
        let result = sqlx::query("DELETE FROM chat_sessions WHERE id = ?")
            .bind(id.to_string())
            .execute(&self.pool)
            .await
            .context("Failed to delete chat session")?;

        Ok(result.rows_affected() > 0)
    }

    pub async fn get_by_provider(&self, provider: &LlmProvider) -> AnyhowResult<Vec<ChatSession>> {
        let rows = sqlx::query(
            r#"
            SELECT id, provider, project_name, start_time, end_time,
                   message_count, token_count, file_path, file_hash,
                   created_at, updated_at, state
            FROM chat_sessions WHERE provider = ? ORDER BY updated_at DESC
            "#,
        )
        .bind(provider.to_string())
        .fetch_all(&self.pool)
        .await
        .context("Failed to fetch chat sessions by provider")?;

        let mut sessions = Vec::new();
        for row in rows {
            let session = self.row_to_session(&row)?;
            sessions.push(session);
        }

        Ok(sessions)
    }

    pub async fn get_by_project_name(&self, project_name: &str) -> AnyhowResult<Vec<ChatSession>> {
        let rows = sqlx::query(
            r#"
            SELECT id, provider, project_name, start_time, end_time,
                   message_count, token_count, file_path, file_hash,
                   created_at, updated_at, state
            FROM chat_sessions WHERE project_name = ? ORDER BY updated_at DESC
            "#,
        )
        .bind(project_name)
        .fetch_all(&self.pool)
        .await
        .context("Failed to fetch chat sessions by project name")?;

        let mut sessions = Vec::new();
        for row in rows {
            let session = self.row_to_session(&row)?;
            sessions.push(session);
        }

        Ok(sessions)
    }

    pub async fn get_by_file_hash(&self, file_hash: &str) -> AnyhowResult<Option<ChatSession>> {
        let row = sqlx::query(
            r#"
            SELECT id, provider, project_name, start_time, end_time,
                   message_count, token_count, file_path, file_hash,
                   created_at, updated_at, state
            FROM chat_sessions WHERE file_hash = ?
            "#,
        )
        .bind(file_hash)
        .fetch_optional(&self.pool)
        .await
        .context("Failed to fetch chat session by file hash")?;

        match row {
            Some(row) => {
                let session = self.row_to_session(&row)?;
                Ok(Some(session))
            }
            None => Ok(None),
        }
    }

    pub async fn count(&self) -> AnyhowResult<i64> {
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM chat_sessions")
            .fetch_one(&self.pool)
            .await
            .context("Failed to count chat sessions")?;

        Ok(count)
    }

    pub async fn count_by_provider(&self, provider: &LlmProvider) -> AnyhowResult<i64> {
        let count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM chat_sessions WHERE provider = ?")
                .bind(provider.to_string())
                .fetch_one(&self.pool)
                .await
                .context("Failed to count chat sessions by provider")?;

        Ok(count)
    }

    pub async fn get_recent_sessions(&self, limit: i64) -> AnyhowResult<Vec<ChatSession>> {
        let rows = sqlx::query(
            r#"
            SELECT id, provider, project_name, start_time, end_time,
                   message_count, token_count, file_path, file_hash,
                   created_at, updated_at, state
            FROM chat_sessions ORDER BY updated_at DESC LIMIT ?
            "#,
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .context("Failed to fetch recent chat sessions")?;

        let mut sessions = Vec::new();
        for row in rows {
            let session = self.row_to_session(&row)?;
            sessions.push(session);
        }

        Ok(sessions)
    }

    fn row_to_session(&self, row: &SqliteRow) -> AnyhowResult<ChatSession> {
        let id_str: String = row.try_get("id")?;
        let provider_str: String = row.try_get("provider")?;
        let project_name: Option<String> = row.try_get("project_name")?;
        let start_time_str: String = row.try_get("start_time")?;
        let end_time_str: Option<String> = row.try_get("end_time")?;
        let message_count: i64 = row.try_get("message_count")?;
        let token_count: Option<i64> = row.try_get("token_count")?;
        let file_path: String = row.try_get("file_path")?;
        let file_hash: String = row.try_get("file_hash")?;
        let created_at_str: String = row.try_get("created_at")?;
        let updated_at_str: String = row.try_get("updated_at")?;
        let state_str: String = row.try_get("state")?;

        let id = Uuid::parse_str(&id_str).context("Invalid session ID format")?;

        let provider = provider_str
            .parse::<LlmProvider>()
            .map_err(|e| anyhow::anyhow!("Invalid provider: {e}"))?;

        let start_time = DateTime::parse_from_rfc3339(&start_time_str)
            .context("Invalid start_time timestamp format")?
            .with_timezone(&Utc);

        let end_time = if let Some(end_time_str) = end_time_str {
            Some(
                DateTime::parse_from_rfc3339(&end_time_str)
                    .context("Invalid end_time timestamp format")?
                    .with_timezone(&Utc),
            )
        } else {
            None
        };

        let created_at = parse_datetime(&created_at_str)
            .context("Invalid created_at timestamp format")?
            .with_timezone(&Utc);

        let updated_at = parse_datetime(&updated_at_str)
            .context("Invalid updated_at timestamp format")?
            .with_timezone(&Utc);

        let state = state_str
            .parse::<SessionState>()
            .map_err(|e| anyhow::anyhow!("Invalid session state: {e}"))?;

        Ok(ChatSession {
            id,
            provider,
            project_name,
            start_time,
            end_time,
            message_count: message_count as u32,
            token_count: token_count.map(|tc| tc as u32),
            file_path,
            file_hash,
            created_at,
            updated_at,
            state,
        })
    }
}
