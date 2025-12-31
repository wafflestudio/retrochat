use anyhow::{Context, Result as AnyhowResult};
use chrono::{DateTime, Utc};
use sqlx::{sqlite::SqliteRow, Pool, Row, Sqlite};
use uuid::Uuid;

use super::connection::DatabaseManager;
use crate::models::{SessionOutcome, SessionSummary};

pub struct SessionSummaryRepository {
    pool: Pool<Sqlite>,
}

impl SessionSummaryRepository {
    pub fn new(db: &DatabaseManager) -> Self {
        Self {
            pool: db.pool().clone(),
        }
    }

    pub async fn create(&self, summary: &SessionSummary) -> AnyhowResult<()> {
        let key_decisions_json = serde_json::to_string(&summary.key_decisions).ok();
        let technologies_used_json = serde_json::to_string(&summary.technologies_used).ok();
        let files_affected_json = serde_json::to_string(&summary.files_affected).ok();

        sqlx::query(
            r#"
            INSERT INTO session_summaries (
                id, session_id,
                title, summary, primary_goal, outcome,
                key_decisions, technologies_used, files_affected,
                total_turns, total_tool_calls, successful_tool_calls, failed_tool_calls, total_lines_changed,
                model_used, prompt_version, generated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(summary.id.to_string())
        .bind(summary.session_id.to_string())
        .bind(&summary.title)
        .bind(&summary.summary)
        .bind(&summary.primary_goal)
        .bind(summary.outcome.as_ref().map(|o| o.to_string()))
        .bind(key_decisions_json)
        .bind(technologies_used_json)
        .bind(files_affected_json)
        .bind(summary.total_turns)
        .bind(summary.total_tool_calls)
        .bind(summary.successful_tool_calls)
        .bind(summary.failed_tool_calls)
        .bind(summary.total_lines_changed)
        .bind(&summary.model_used)
        .bind(summary.prompt_version)
        .bind(summary.generated_at.to_rfc3339())
        .execute(&self.pool)
        .await
        .context("Failed to create session summary")?;

        Ok(())
    }

    pub async fn get_by_id(&self, id: &Uuid) -> AnyhowResult<Option<SessionSummary>> {
        let row = sqlx::query(
            r#"
            SELECT * FROM session_summaries WHERE id = ?
            "#,
        )
        .bind(id.to_string())
        .fetch_optional(&self.pool)
        .await
        .context("Failed to fetch session summary by ID")?;

        match row {
            Some(row) => {
                let summary = self.row_to_session_summary(&row)?;
                Ok(Some(summary))
            }
            None => Ok(None),
        }
    }

    pub async fn get_by_session(&self, session_id: &Uuid) -> AnyhowResult<Option<SessionSummary>> {
        let row = sqlx::query(
            r#"
            SELECT * FROM session_summaries WHERE session_id = ?
            "#,
        )
        .bind(session_id.to_string())
        .fetch_optional(&self.pool)
        .await
        .context("Failed to fetch session summary by session ID")?;

        match row {
            Some(row) => {
                let summary = self.row_to_session_summary(&row)?;
                Ok(Some(summary))
            }
            None => Ok(None),
        }
    }

    pub async fn get_by_outcome(
        &self,
        outcome: &SessionOutcome,
    ) -> AnyhowResult<Vec<SessionSummary>> {
        let rows = sqlx::query(
            r#"
            SELECT * FROM session_summaries
            WHERE outcome = ?
            ORDER BY generated_at DESC
            "#,
        )
        .bind(outcome.to_string())
        .fetch_all(&self.pool)
        .await
        .context("Failed to fetch session summaries by outcome")?;

        let mut summaries = Vec::new();
        for row in rows {
            let summary = self.row_to_session_summary(&row)?;
            summaries.push(summary);
        }

        Ok(summaries)
    }

    pub async fn get_all(&self, limit: i64, offset: i64) -> AnyhowResult<Vec<SessionSummary>> {
        let rows = sqlx::query(
            r#"
            SELECT * FROM session_summaries
            ORDER BY generated_at DESC
            LIMIT ? OFFSET ?
            "#,
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .context("Failed to fetch session summaries")?;

        let mut summaries = Vec::new();
        for row in rows {
            let summary = self.row_to_session_summary(&row)?;
            summaries.push(summary);
        }

        Ok(summaries)
    }

    pub async fn update(&self, summary: &SessionSummary) -> AnyhowResult<bool> {
        let key_decisions_json = serde_json::to_string(&summary.key_decisions).ok();
        let technologies_used_json = serde_json::to_string(&summary.technologies_used).ok();
        let files_affected_json = serde_json::to_string(&summary.files_affected).ok();

        let result = sqlx::query(
            r#"
            UPDATE session_summaries SET
                title = ?,
                summary = ?,
                primary_goal = ?,
                outcome = ?,
                key_decisions = ?,
                technologies_used = ?,
                files_affected = ?,
                total_turns = ?,
                total_tool_calls = ?,
                successful_tool_calls = ?,
                failed_tool_calls = ?,
                total_lines_changed = ?,
                model_used = ?,
                prompt_version = ?,
                generated_at = ?
            WHERE id = ?
            "#,
        )
        .bind(&summary.title)
        .bind(&summary.summary)
        .bind(&summary.primary_goal)
        .bind(summary.outcome.as_ref().map(|o| o.to_string()))
        .bind(key_decisions_json)
        .bind(technologies_used_json)
        .bind(files_affected_json)
        .bind(summary.total_turns)
        .bind(summary.total_tool_calls)
        .bind(summary.successful_tool_calls)
        .bind(summary.failed_tool_calls)
        .bind(summary.total_lines_changed)
        .bind(&summary.model_used)
        .bind(summary.prompt_version)
        .bind(summary.generated_at.to_rfc3339())
        .bind(summary.id.to_string())
        .execute(&self.pool)
        .await
        .context("Failed to update session summary")?;

        Ok(result.rows_affected() > 0)
    }

    pub async fn delete(&self, id: &Uuid) -> AnyhowResult<bool> {
        let result = sqlx::query(
            r#"
            DELETE FROM session_summaries WHERE id = ?
            "#,
        )
        .bind(id.to_string())
        .execute(&self.pool)
        .await
        .context("Failed to delete session summary")?;

        Ok(result.rows_affected() > 0)
    }

    pub async fn delete_by_session(&self, session_id: &Uuid) -> AnyhowResult<bool> {
        let result = sqlx::query(
            r#"
            DELETE FROM session_summaries WHERE session_id = ?
            "#,
        )
        .bind(session_id.to_string())
        .execute(&self.pool)
        .await
        .context("Failed to delete session summary by session ID")?;

        Ok(result.rows_affected() > 0)
    }

    /// Search session summaries using FTS
    pub async fn search(&self, query: &str) -> AnyhowResult<Vec<SessionSummary>> {
        let rows = sqlx::query(
            r#"
            SELECT ss.* FROM session_summaries ss
            JOIN session_summaries_fts fts ON ss.rowid = fts.rowid
            WHERE session_summaries_fts MATCH ?
            ORDER BY rank
            "#,
        )
        .bind(query)
        .fetch_all(&self.pool)
        .await
        .context("Failed to search session summaries")?;

        let mut summaries = Vec::new();
        for row in rows {
            let summary = self.row_to_session_summary(&row)?;
            summaries.push(summary);
        }

        Ok(summaries)
    }

    /// Get sessions that don't have summaries yet
    pub async fn get_sessions_without_summaries(&self) -> AnyhowResult<Vec<Uuid>> {
        let rows = sqlx::query(
            r#"
            SELECT cs.id FROM chat_sessions cs
            LEFT JOIN session_summaries ss ON cs.id = ss.session_id
            WHERE ss.id IS NULL
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .context("Failed to fetch sessions without summaries")?;

        let mut session_ids = Vec::new();
        for row in rows {
            let id_str: String = row.try_get("id")?;
            let id = Uuid::parse_str(&id_str).context("Invalid session ID format")?;
            session_ids.push(id);
        }

        Ok(session_ids)
    }

    /// Count sessions without summaries
    pub async fn count_pending_summaries(&self) -> AnyhowResult<i64> {
        let count: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*) FROM chat_sessions cs
            LEFT JOIN session_summaries ss ON cs.id = ss.session_id
            WHERE ss.id IS NULL
            "#,
        )
        .fetch_one(&self.pool)
        .await
        .context("Failed to count pending session summaries")?;

        Ok(count)
    }

    /// Get sessions with significant changes (for prioritizing summarization)
    pub async fn get_sessions_with_significant_changes(
        &self,
        min_lines_changed: i32,
    ) -> AnyhowResult<Vec<SessionSummary>> {
        let rows = sqlx::query(
            r#"
            SELECT * FROM session_summaries
            WHERE total_lines_changed >= ?
            ORDER BY total_lines_changed DESC
            "#,
        )
        .bind(min_lines_changed)
        .fetch_all(&self.pool)
        .await
        .context("Failed to fetch sessions with significant changes")?;

        let mut summaries = Vec::new();
        for row in rows {
            let summary = self.row_to_session_summary(&row)?;
            summaries.push(summary);
        }

        Ok(summaries)
    }

    fn row_to_session_summary(&self, row: &SqliteRow) -> AnyhowResult<SessionSummary> {
        let id_str: String = row.try_get("id")?;
        let session_id_str: String = row.try_get("session_id")?;
        let generated_at_str: String = row.try_get("generated_at")?;

        let id = Uuid::parse_str(&id_str).context("Invalid summary ID format")?;
        let session_id = Uuid::parse_str(&session_id_str).context("Invalid session ID format")?;
        let generated_at = DateTime::parse_from_rfc3339(&generated_at_str)
            .context("Invalid generated_at format")?
            .with_timezone(&Utc);

        let outcome_str: Option<String> = row.try_get("outcome").ok();
        let outcome = outcome_str.and_then(|s| s.parse().ok());

        // Parse JSON arrays
        let key_decisions_json: Option<String> = row.try_get("key_decisions").ok();
        let key_decisions: Vec<String> = key_decisions_json
            .and_then(|json| serde_json::from_str(&json).ok())
            .unwrap_or_default();

        let technologies_used_json: Option<String> = row.try_get("technologies_used").ok();
        let technologies_used: Vec<String> = technologies_used_json
            .and_then(|json| serde_json::from_str(&json).ok())
            .unwrap_or_default();

        let files_affected_json: Option<String> = row.try_get("files_affected").ok();
        let files_affected: Vec<String> = files_affected_json
            .and_then(|json| serde_json::from_str(&json).ok())
            .unwrap_or_default();

        Ok(SessionSummary {
            id,
            session_id,
            title: row.try_get("title")?,
            summary: row.try_get("summary")?,
            primary_goal: row.try_get("primary_goal").ok(),
            outcome,
            key_decisions,
            technologies_used,
            files_affected,
            total_turns: row.try_get("total_turns")?,
            total_tool_calls: row.try_get("total_tool_calls")?,
            successful_tool_calls: row.try_get("successful_tool_calls")?,
            failed_tool_calls: row.try_get("failed_tool_calls")?,
            total_lines_changed: row.try_get("total_lines_changed")?,
            model_used: row.try_get("model_used").ok(),
            prompt_version: row.try_get("prompt_version")?,
            generated_at,
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
    async fn test_create_and_get_session_summary() {
        let (db, session_id) = setup_db().await;
        let repo = SessionSummaryRepository::new(&db);

        let summary = SessionSummary::new(
            session_id,
            "JWT Implementation".to_string(),
            "Implemented JWT authentication".to_string(),
        )
        .with_outcome(SessionOutcome::Completed)
        .with_metrics(5, 20, 18, 2, 500);

        repo.create(&summary).await.unwrap();

        let retrieved = repo.get_by_id(&summary.id).await.unwrap();
        assert!(retrieved.is_some());

        let retrieved = retrieved.unwrap();
        assert_eq!(retrieved.session_id, session_id);
        assert_eq!(retrieved.title, "JWT Implementation");
        assert_eq!(retrieved.outcome, Some(SessionOutcome::Completed));
        assert_eq!(retrieved.total_turns, 5);
        assert_eq!(retrieved.total_lines_changed, 500);
    }

    #[tokio::test]
    async fn test_get_by_session() {
        let (db, session_id) = setup_db().await;
        let repo = SessionSummaryRepository::new(&db);

        let summary = SessionSummary::new(session_id, "Title".to_string(), "Summary".to_string());
        repo.create(&summary).await.unwrap();

        let retrieved = repo.get_by_session(&session_id).await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().session_id, session_id);
    }

    #[tokio::test]
    async fn test_json_fields_serialization() {
        let (db, session_id) = setup_db().await;
        let repo = SessionSummaryRepository::new(&db);

        let summary = SessionSummary::new(session_id, "Title".to_string(), "Summary".to_string())
            .with_key_decisions(vec!["Use JWT".to_string(), "RS256".to_string()])
            .with_technologies(vec!["Rust".to_string(), "axum".to_string()])
            .with_files_affected(vec!["src/auth.rs".to_string()]);

        repo.create(&summary).await.unwrap();

        let retrieved = repo.get_by_session(&session_id).await.unwrap().unwrap();
        assert_eq!(retrieved.key_decisions.len(), 2);
        assert_eq!(retrieved.technologies_used.len(), 2);
        assert_eq!(retrieved.files_affected.len(), 1);
    }

    #[tokio::test]
    async fn test_update() {
        let (db, session_id) = setup_db().await;
        let repo = SessionSummaryRepository::new(&db);

        let mut summary = SessionSummary::new(
            session_id,
            "Old Title".to_string(),
            "Old Summary".to_string(),
        );
        repo.create(&summary).await.unwrap();

        summary.title = "New Title".to_string();
        summary.outcome = Some(SessionOutcome::Partial);

        let updated = repo.update(&summary).await.unwrap();
        assert!(updated);

        let retrieved = repo.get_by_id(&summary.id).await.unwrap().unwrap();
        assert_eq!(retrieved.title, "New Title");
        assert_eq!(retrieved.outcome, Some(SessionOutcome::Partial));
    }

    #[tokio::test]
    async fn test_get_sessions_without_summaries() {
        let (db, session_id) = setup_db().await;
        let session_repo = ChatSessionRepository::new(&db);
        let summary_repo = SessionSummaryRepository::new(&db);

        // Create another session without a summary
        let session_id2 = Uuid::new_v4();
        let mut session2 = ChatSession::new(
            Provider::ClaudeCode,
            "/test/file2.jsonl".to_string(),
            "test_hash2".to_string(),
            Utc::now(),
        );
        session2.id = session_id2;
        session2.set_state(SessionState::Imported);
        session_repo.create(&session2).await.unwrap();

        // Create summary only for first session
        let summary = SessionSummary::new(session_id, "T".to_string(), "S".to_string());
        summary_repo.create(&summary).await.unwrap();

        let sessions_without = summary_repo.get_sessions_without_summaries().await.unwrap();
        assert_eq!(sessions_without.len(), 1);
        assert_eq!(sessions_without[0], session_id2);
    }

    #[tokio::test]
    async fn test_get_by_outcome() {
        let (db, session_id) = setup_db().await;
        let session_repo = ChatSessionRepository::new(&db);
        let summary_repo = SessionSummaryRepository::new(&db);

        // Create another session
        let session_id2 = Uuid::new_v4();
        let mut session2 = ChatSession::new(
            Provider::ClaudeCode,
            "/test/file2.jsonl".to_string(),
            "test_hash2".to_string(),
            Utc::now(),
        );
        session2.id = session_id2;
        session2.set_state(SessionState::Imported);
        session_repo.create(&session2).await.unwrap();

        // Create summaries with different outcomes
        let summary1 = SessionSummary::new(session_id, "T1".to_string(), "S1".to_string())
            .with_outcome(SessionOutcome::Completed);
        let summary2 = SessionSummary::new(session_id2, "T2".to_string(), "S2".to_string())
            .with_outcome(SessionOutcome::Abandoned);

        summary_repo.create(&summary1).await.unwrap();
        summary_repo.create(&summary2).await.unwrap();

        let completed = summary_repo
            .get_by_outcome(&SessionOutcome::Completed)
            .await
            .unwrap();
        assert_eq!(completed.len(), 1);

        let abandoned = summary_repo
            .get_by_outcome(&SessionOutcome::Abandoned)
            .await
            .unwrap();
        assert_eq!(abandoned.len(), 1);
    }
}
