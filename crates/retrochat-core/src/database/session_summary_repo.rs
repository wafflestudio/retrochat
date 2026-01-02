use anyhow::{Context, Result as AnyhowResult};
use chrono::{DateTime, Utc};
use sqlx::{Pool, Row, Sqlite};

use super::connection::DatabaseManager;
use crate::models::session_summary::{SessionOutcome, SessionSummary};

pub struct SessionSummaryRepository {
    pool: Pool<Sqlite>,
}

impl SessionSummaryRepository {
    pub fn new(db: &DatabaseManager) -> Self {
        Self {
            pool: db.pool().clone(),
        }
    }

    /// Create a new session summary
    pub async fn create(&self, summary: &SessionSummary) -> AnyhowResult<String> {
        let generated_at = summary.generated_at.to_rfc3339();
        let outcome = summary.outcome.as_ref().map(|o| o.to_string());

        // Serialize JSON arrays
        let key_decisions_json = summary
            .key_decisions
            .as_ref()
            .map(serde_json::to_string)
            .transpose()
            .context("Failed to serialize key_decisions")?;
        let technologies_used_json = summary
            .technologies_used
            .as_ref()
            .map(serde_json::to_string)
            .transpose()
            .context("Failed to serialize technologies_used")?;
        let files_affected_json = summary
            .files_affected
            .as_ref()
            .map(serde_json::to_string)
            .transpose()
            .context("Failed to serialize files_affected")?;

        sqlx::query(
            r#"
            INSERT INTO session_summaries (
                id, session_id,
                title, summary, primary_goal, outcome,
                key_decisions, technologies_used, files_affected,
                model_used, prompt_version, generated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&summary.id)
        .bind(&summary.session_id)
        .bind(&summary.title)
        .bind(&summary.summary)
        .bind(&summary.primary_goal)
        .bind(&outcome)
        .bind(&key_decisions_json)
        .bind(&technologies_used_json)
        .bind(&files_affected_json)
        .bind(&summary.model_used)
        .bind(summary.prompt_version)
        .bind(&generated_at)
        .execute(&self.pool)
        .await
        .context("Failed to insert session summary")?;

        Ok(summary.id.clone())
    }

    /// Update an existing session summary
    pub async fn update(&self, summary: &SessionSummary) -> AnyhowResult<()> {
        let generated_at = summary.generated_at.to_rfc3339();
        let outcome = summary.outcome.as_ref().map(|o| o.to_string());

        // Serialize JSON arrays
        let key_decisions_json = summary
            .key_decisions
            .as_ref()
            .map(serde_json::to_string)
            .transpose()
            .context("Failed to serialize key_decisions")?;
        let technologies_used_json = summary
            .technologies_used
            .as_ref()
            .map(serde_json::to_string)
            .transpose()
            .context("Failed to serialize technologies_used")?;
        let files_affected_json = summary
            .files_affected
            .as_ref()
            .map(serde_json::to_string)
            .transpose()
            .context("Failed to serialize files_affected")?;

        sqlx::query(
            r#"
            UPDATE session_summaries SET
                title = ?, summary = ?, primary_goal = ?, outcome = ?,
                key_decisions = ?, technologies_used = ?, files_affected = ?,
                model_used = ?, prompt_version = ?, generated_at = ?
            WHERE id = ?
            "#,
        )
        .bind(&summary.title)
        .bind(&summary.summary)
        .bind(&summary.primary_goal)
        .bind(&outcome)
        .bind(&key_decisions_json)
        .bind(&technologies_used_json)
        .bind(&files_affected_json)
        .bind(&summary.model_used)
        .bind(summary.prompt_version)
        .bind(&generated_at)
        .bind(&summary.id)
        .execute(&self.pool)
        .await
        .context("Failed to update session summary")?;

        Ok(())
    }

    /// Get a session summary by ID
    pub async fn get_by_id(&self, id: &str) -> AnyhowResult<Option<SessionSummary>> {
        let row = sqlx::query(
            r#"
            SELECT
                id, session_id,
                title, summary, primary_goal, outcome,
                key_decisions, technologies_used, files_affected,
                model_used, prompt_version, generated_at
            FROM session_summaries
            WHERE id = ?
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .context("Failed to fetch session summary")?;

        row.map(|r| Self::row_to_session_summary(&r)).transpose()
    }

    /// Get a session summary by session ID
    pub async fn get_by_session(&self, session_id: &str) -> AnyhowResult<Option<SessionSummary>> {
        let row = sqlx::query(
            r#"
            SELECT
                id, session_id,
                title, summary, primary_goal, outcome,
                key_decisions, technologies_used, files_affected,
                model_used, prompt_version, generated_at
            FROM session_summaries
            WHERE session_id = ?
            "#,
        )
        .bind(session_id)
        .fetch_optional(&self.pool)
        .await
        .context("Failed to fetch session summary")?;

        row.map(|r| Self::row_to_session_summary(&r)).transpose()
    }

    /// Check if a session has a summary
    pub async fn exists_for_session(&self, session_id: &str) -> AnyhowResult<bool> {
        let row = sqlx::query(
            r#"
            SELECT COUNT(*) as count FROM session_summaries
            WHERE session_id = ?
            "#,
        )
        .bind(session_id)
        .fetch_one(&self.pool)
        .await
        .context("Failed to check session summary existence")?;

        let count: i64 = row.get("count");
        Ok(count > 0)
    }

    /// Delete a session summary by session ID
    pub async fn delete_by_session(&self, session_id: &str) -> AnyhowResult<u64> {
        let result = sqlx::query(
            r#"
            DELETE FROM session_summaries
            WHERE session_id = ?
            "#,
        )
        .bind(session_id)
        .execute(&self.pool)
        .await
        .context("Failed to delete session summary")?;

        Ok(result.rows_affected())
    }

    /// Search session summaries using full-text search
    pub async fn search(&self, query: &str, limit: i64) -> AnyhowResult<Vec<SessionSummary>> {
        let rows = sqlx::query(
            r#"
            SELECT
                ss.id, ss.session_id,
                ss.title, ss.summary, ss.primary_goal, ss.outcome,
                ss.key_decisions, ss.technologies_used, ss.files_affected,
                ss.model_used, ss.prompt_version, ss.generated_at
            FROM session_summaries ss
            JOIN session_summaries_fts fts ON ss.rowid = fts.rowid
            WHERE session_summaries_fts MATCH ?
            ORDER BY rank
            LIMIT ?
            "#,
        )
        .bind(query)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .context("Failed to search session summaries")?;

        rows.iter().map(Self::row_to_session_summary).collect()
    }

    /// List all session summaries with optional outcome filter
    pub async fn list_all(
        &self,
        outcome: Option<&SessionOutcome>,
        limit: i64,
        offset: i64,
    ) -> AnyhowResult<Vec<SessionSummary>> {
        let rows = if let Some(outcome) = outcome {
            let outcome_str = outcome.to_string();
            sqlx::query(
                r#"
                SELECT
                    id, session_id,
                    title, summary, primary_goal, outcome,
                    key_decisions, technologies_used, files_affected,
                    model_used, prompt_version, generated_at
                FROM session_summaries
                WHERE outcome = ?
                ORDER BY generated_at DESC
                LIMIT ? OFFSET ?
                "#,
            )
            .bind(&outcome_str)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await
            .context("Failed to list session summaries")?
        } else {
            sqlx::query(
                r#"
                SELECT
                    id, session_id,
                    title, summary, primary_goal, outcome,
                    key_decisions, technologies_used, files_affected,
                    model_used, prompt_version, generated_at
                FROM session_summaries
                ORDER BY generated_at DESC
                LIMIT ? OFFSET ?
                "#,
            )
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await
            .context("Failed to list session summaries")?
        };

        rows.iter().map(Self::row_to_session_summary).collect()
    }

    /// Convert a database row to SessionSummary
    fn row_to_session_summary(row: &sqlx::sqlite::SqliteRow) -> AnyhowResult<SessionSummary> {
        let generated_at_str: String = row.get("generated_at");
        let generated_at = DateTime::parse_from_rfc3339(&generated_at_str)?.with_timezone(&Utc);

        let outcome_str: Option<String> = row.get("outcome");
        let outcome = outcome_str
            .map(|o| o.parse::<SessionOutcome>())
            .transpose()
            .ok()
            .flatten();

        let key_decisions_json: Option<String> = row.get("key_decisions");
        let key_decisions: Option<Vec<String>> = key_decisions_json
            .map(|d| serde_json::from_str(&d))
            .transpose()
            .context("Failed to deserialize key_decisions")?;

        let technologies_used_json: Option<String> = row.get("technologies_used");
        let technologies_used: Option<Vec<String>> = technologies_used_json
            .map(|t| serde_json::from_str(&t))
            .transpose()
            .context("Failed to deserialize technologies_used")?;

        let files_affected_json: Option<String> = row.get("files_affected");
        let files_affected: Option<Vec<String>> = files_affected_json
            .map(|f| serde_json::from_str(&f))
            .transpose()
            .context("Failed to deserialize files_affected")?;

        Ok(SessionSummary {
            id: row.get("id"),
            session_id: row.get("session_id"),
            title: row.get("title"),
            summary: row.get("summary"),
            primary_goal: row.get("primary_goal"),
            outcome,
            key_decisions,
            technologies_used,
            files_affected,
            model_used: row.get("model_used"),
            prompt_version: row.get("prompt_version"),
            generated_at,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::DatabaseManager;

    #[tokio::test]
    async fn test_create_and_get_session_summary() {
        let db = DatabaseManager::open_in_memory().await.unwrap();
        let repo = SessionSummaryRepository::new(&db);

        // Create test session first
        sqlx::query(
            r#"
            INSERT INTO chat_sessions (id, provider, project_name, start_time, end_time, message_count, file_path, file_hash, state)
            VALUES ('session-1', 'ClaudeCode', NULL, '2024-01-01T00:00:00Z', '2024-01-01T01:00:00Z', 5, '/test.jsonl', 'hash1', 'Imported')
            "#,
        )
        .execute(db.pool())
        .await
        .unwrap();

        let summary = SessionSummary::new(
            "session-1".to_string(),
            "JWT Authentication Implementation".to_string(),
            "Implemented JWT auth for the API".to_string(),
        )
        .with_outcome(SessionOutcome::Completed)
        .with_technologies_used(vec!["JWT".to_string(), "bcrypt".to_string()]);

        let id = repo.create(&summary).await.unwrap();
        assert!(!id.is_empty());

        let fetched = repo.get_by_id(&id).await.unwrap().unwrap();
        assert_eq!(fetched.session_id, "session-1");
        assert_eq!(fetched.title, "JWT Authentication Implementation");
        assert_eq!(fetched.outcome, Some(SessionOutcome::Completed));
        assert_eq!(
            fetched.technologies_used,
            Some(vec!["JWT".to_string(), "bcrypt".to_string()])
        );
    }

    #[tokio::test]
    async fn test_get_by_session() {
        let db = DatabaseManager::open_in_memory().await.unwrap();
        let repo = SessionSummaryRepository::new(&db);

        // Create test session
        sqlx::query(
            r#"
            INSERT INTO chat_sessions (id, provider, project_name, start_time, end_time, message_count, file_path, file_hash, state)
            VALUES ('session-2', 'ClaudeCode', NULL, '2024-01-01T00:00:00Z', '2024-01-01T01:00:00Z', 10, '/test2.jsonl', 'hash2', 'Imported')
            "#,
        )
        .execute(db.pool())
        .await
        .unwrap();

        let summary = SessionSummary::new(
            "session-2".to_string(),
            "Title".to_string(),
            "Summary".to_string(),
        );
        repo.create(&summary).await.unwrap();

        let fetched = repo.get_by_session("session-2").await.unwrap().unwrap();
        assert_eq!(fetched.session_id, "session-2");
        assert_eq!(fetched.title, "Title");
    }

    #[tokio::test]
    async fn test_exists_for_session() {
        let db = DatabaseManager::open_in_memory().await.unwrap();
        let repo = SessionSummaryRepository::new(&db);

        // Create test session
        sqlx::query(
            r#"
            INSERT INTO chat_sessions (id, provider, project_name, start_time, end_time, message_count, file_path, file_hash, state)
            VALUES ('session-3', 'ClaudeCode', NULL, '2024-01-01T00:00:00Z', '2024-01-01T01:00:00Z', 5, '/test3.jsonl', 'hash3', 'Imported')
            "#,
        )
        .execute(db.pool())
        .await
        .unwrap();

        assert!(!repo.exists_for_session("session-3").await.unwrap());

        let summary = SessionSummary::new(
            "session-3".to_string(),
            "Title".to_string(),
            "Summary".to_string(),
        );
        repo.create(&summary).await.unwrap();

        assert!(repo.exists_for_session("session-3").await.unwrap());
    }

    #[tokio::test]
    async fn test_delete_by_session() {
        let db = DatabaseManager::open_in_memory().await.unwrap();
        let repo = SessionSummaryRepository::new(&db);

        // Create test session
        sqlx::query(
            r#"
            INSERT INTO chat_sessions (id, provider, project_name, start_time, end_time, message_count, file_path, file_hash, state)
            VALUES ('session-4', 'ClaudeCode', NULL, '2024-01-01T00:00:00Z', '2024-01-01T01:00:00Z', 5, '/test4.jsonl', 'hash4', 'Imported')
            "#,
        )
        .execute(db.pool())
        .await
        .unwrap();

        let summary = SessionSummary::new(
            "session-4".to_string(),
            "Title".to_string(),
            "Summary".to_string(),
        );
        repo.create(&summary).await.unwrap();

        let deleted = repo.delete_by_session("session-4").await.unwrap();
        assert_eq!(deleted, 1);

        assert!(!repo.exists_for_session("session-4").await.unwrap());
    }

    #[tokio::test]
    async fn test_update_session_summary() {
        let db = DatabaseManager::open_in_memory().await.unwrap();
        let repo = SessionSummaryRepository::new(&db);

        // Create test session
        sqlx::query(
            r#"
            INSERT INTO chat_sessions (id, provider, project_name, start_time, end_time, message_count, file_path, file_hash, state)
            VALUES ('session-5', 'ClaudeCode', NULL, '2024-01-01T00:00:00Z', '2024-01-01T01:00:00Z', 5, '/test5.jsonl', 'hash5', 'Imported')
            "#,
        )
        .execute(db.pool())
        .await
        .unwrap();

        let mut summary = SessionSummary::new(
            "session-5".to_string(),
            "Original Title".to_string(),
            "Original Summary".to_string(),
        );
        let id = repo.create(&summary).await.unwrap();

        summary.title = "Updated Title".to_string();
        summary.outcome = Some(SessionOutcome::Completed);
        repo.update(&summary).await.unwrap();

        let fetched = repo.get_by_id(&id).await.unwrap().unwrap();
        assert_eq!(fetched.title, "Updated Title");
        assert_eq!(fetched.outcome, Some(SessionOutcome::Completed));
    }
}
