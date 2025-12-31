use anyhow::{Context, Result as AnyhowResult};
use chrono::{DateTime, Utc};
use sqlx::{sqlite::SqliteRow, Pool, Row, Sqlite};
use uuid::Uuid;

use super::connection::DatabaseManager;
use crate::models::{TurnSummary, TurnType};

pub struct TurnSummaryRepository {
    pool: Pool<Sqlite>,
}

impl TurnSummaryRepository {
    pub fn new(db: &DatabaseManager) -> Self {
        Self {
            pool: db.pool().clone(),
        }
    }

    pub async fn create(&self, summary: &TurnSummary) -> AnyhowResult<()> {
        sqlx::query(
            r#"
            INSERT INTO turn_summaries (
                id, turn_id,
                user_intent, assistant_action, summary,
                turn_type, complexity_score,
                model_used, prompt_version, generated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(summary.id.to_string())
        .bind(summary.turn_id.to_string())
        .bind(&summary.user_intent)
        .bind(&summary.assistant_action)
        .bind(&summary.summary)
        .bind(summary.turn_type.as_ref().map(|t| t.to_string()))
        .bind(summary.complexity_score)
        .bind(&summary.model_used)
        .bind(summary.prompt_version)
        .bind(summary.generated_at.to_rfc3339())
        .execute(&self.pool)
        .await
        .context("Failed to create turn summary")?;

        Ok(())
    }

    pub async fn bulk_create(&self, summaries: &[TurnSummary]) -> AnyhowResult<()> {
        if summaries.is_empty() {
            return Ok(());
        }

        let mut tx = self.pool.begin().await?;

        for summary in summaries {
            sqlx::query(
                r#"
                INSERT INTO turn_summaries (
                    id, turn_id,
                    user_intent, assistant_action, summary,
                    turn_type, complexity_score,
                    model_used, prompt_version, generated_at
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                "#,
            )
            .bind(summary.id.to_string())
            .bind(summary.turn_id.to_string())
            .bind(&summary.user_intent)
            .bind(&summary.assistant_action)
            .bind(&summary.summary)
            .bind(summary.turn_type.as_ref().map(|t| t.to_string()))
            .bind(summary.complexity_score)
            .bind(&summary.model_used)
            .bind(summary.prompt_version)
            .bind(summary.generated_at.to_rfc3339())
            .execute(&mut *tx)
            .await
            .context("Failed to create turn summary in bulk")?;
        }

        tx.commit().await?;
        Ok(())
    }

    pub async fn get_by_id(&self, id: &Uuid) -> AnyhowResult<Option<TurnSummary>> {
        let row = sqlx::query(
            r#"
            SELECT * FROM turn_summaries WHERE id = ?
            "#,
        )
        .bind(id.to_string())
        .fetch_optional(&self.pool)
        .await
        .context("Failed to fetch turn summary by ID")?;

        match row {
            Some(row) => {
                let summary = self.row_to_turn_summary(&row)?;
                Ok(Some(summary))
            }
            None => Ok(None),
        }
    }

    pub async fn get_by_turn(&self, turn_id: &Uuid) -> AnyhowResult<Option<TurnSummary>> {
        let row = sqlx::query(
            r#"
            SELECT * FROM turn_summaries WHERE turn_id = ?
            "#,
        )
        .bind(turn_id.to_string())
        .fetch_optional(&self.pool)
        .await
        .context("Failed to fetch turn summary by turn ID")?;

        match row {
            Some(row) => {
                let summary = self.row_to_turn_summary(&row)?;
                Ok(Some(summary))
            }
            None => Ok(None),
        }
    }

    pub async fn get_by_session(&self, session_id: &Uuid) -> AnyhowResult<Vec<TurnSummary>> {
        let rows = sqlx::query(
            r#"
            SELECT ts.* FROM turn_summaries ts
            JOIN detected_turns dt ON ts.turn_id = dt.id
            WHERE dt.session_id = ?
            ORDER BY dt.turn_number ASC
            "#,
        )
        .bind(session_id.to_string())
        .fetch_all(&self.pool)
        .await
        .context("Failed to fetch turn summaries by session")?;

        let mut summaries = Vec::new();
        for row in rows {
            let summary = self.row_to_turn_summary(&row)?;
            summaries.push(summary);
        }

        Ok(summaries)
    }

    pub async fn get_by_turn_type(&self, turn_type: &TurnType) -> AnyhowResult<Vec<TurnSummary>> {
        let rows = sqlx::query(
            r#"
            SELECT * FROM turn_summaries
            WHERE turn_type = ?
            ORDER BY generated_at DESC
            "#,
        )
        .bind(turn_type.to_string())
        .fetch_all(&self.pool)
        .await
        .context("Failed to fetch turn summaries by type")?;

        let mut summaries = Vec::new();
        for row in rows {
            let summary = self.row_to_turn_summary(&row)?;
            summaries.push(summary);
        }

        Ok(summaries)
    }

    pub async fn update(&self, summary: &TurnSummary) -> AnyhowResult<bool> {
        let result = sqlx::query(
            r#"
            UPDATE turn_summaries SET
                user_intent = ?,
                assistant_action = ?,
                summary = ?,
                turn_type = ?,
                complexity_score = ?,
                model_used = ?,
                prompt_version = ?,
                generated_at = ?
            WHERE id = ?
            "#,
        )
        .bind(&summary.user_intent)
        .bind(&summary.assistant_action)
        .bind(&summary.summary)
        .bind(summary.turn_type.as_ref().map(|t| t.to_string()))
        .bind(summary.complexity_score)
        .bind(&summary.model_used)
        .bind(summary.prompt_version)
        .bind(summary.generated_at.to_rfc3339())
        .bind(summary.id.to_string())
        .execute(&self.pool)
        .await
        .context("Failed to update turn summary")?;

        Ok(result.rows_affected() > 0)
    }

    pub async fn delete(&self, id: &Uuid) -> AnyhowResult<bool> {
        let result = sqlx::query(
            r#"
            DELETE FROM turn_summaries WHERE id = ?
            "#,
        )
        .bind(id.to_string())
        .execute(&self.pool)
        .await
        .context("Failed to delete turn summary")?;

        Ok(result.rows_affected() > 0)
    }

    pub async fn delete_by_turn(&self, turn_id: &Uuid) -> AnyhowResult<bool> {
        let result = sqlx::query(
            r#"
            DELETE FROM turn_summaries WHERE turn_id = ?
            "#,
        )
        .bind(turn_id.to_string())
        .execute(&self.pool)
        .await
        .context("Failed to delete turn summary by turn ID")?;

        Ok(result.rows_affected() > 0)
    }

    /// Search turn summaries using FTS
    pub async fn search(&self, query: &str) -> AnyhowResult<Vec<TurnSummary>> {
        let rows = sqlx::query(
            r#"
            SELECT ts.* FROM turn_summaries ts
            JOIN turn_summaries_fts fts ON ts.rowid = fts.rowid
            WHERE turn_summaries_fts MATCH ?
            ORDER BY rank
            "#,
        )
        .bind(query)
        .fetch_all(&self.pool)
        .await
        .context("Failed to search turn summaries")?;

        let mut summaries = Vec::new();
        for row in rows {
            let summary = self.row_to_turn_summary(&row)?;
            summaries.push(summary);
        }

        Ok(summaries)
    }

    /// Get turns that don't have summaries yet (for a specific session)
    pub async fn get_turns_without_summaries(&self, session_id: &Uuid) -> AnyhowResult<Vec<Uuid>> {
        let rows = sqlx::query(
            r#"
            SELECT dt.id FROM detected_turns dt
            LEFT JOIN turn_summaries ts ON dt.id = ts.turn_id
            WHERE dt.session_id = ? AND ts.id IS NULL
            ORDER BY dt.turn_number ASC
            "#,
        )
        .bind(session_id.to_string())
        .fetch_all(&self.pool)
        .await
        .context("Failed to fetch turns without summaries")?;

        let mut turn_ids = Vec::new();
        for row in rows {
            let id_str: String = row.try_get("id")?;
            let id = Uuid::parse_str(&id_str).context("Invalid turn ID format")?;
            turn_ids.push(id);
        }

        Ok(turn_ids)
    }

    /// Count turns without summaries across all sessions
    pub async fn count_pending_summaries(&self) -> AnyhowResult<i64> {
        let count: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*) FROM detected_turns dt
            LEFT JOIN turn_summaries ts ON dt.id = ts.turn_id
            WHERE ts.id IS NULL
            "#,
        )
        .fetch_one(&self.pool)
        .await
        .context("Failed to count pending turn summaries")?;

        Ok(count)
    }

    fn row_to_turn_summary(&self, row: &SqliteRow) -> AnyhowResult<TurnSummary> {
        let id_str: String = row.try_get("id")?;
        let turn_id_str: String = row.try_get("turn_id")?;
        let generated_at_str: String = row.try_get("generated_at")?;

        let id = Uuid::parse_str(&id_str).context("Invalid summary ID format")?;
        let turn_id = Uuid::parse_str(&turn_id_str).context("Invalid turn ID format")?;
        let generated_at = DateTime::parse_from_rfc3339(&generated_at_str)
            .context("Invalid generated_at format")?
            .with_timezone(&Utc);

        let turn_type_str: Option<String> = row.try_get("turn_type").ok();
        let turn_type = turn_type_str.and_then(|s| s.parse().ok());

        Ok(TurnSummary {
            id,
            turn_id,
            user_intent: row.try_get("user_intent")?,
            assistant_action: row.try_get("assistant_action")?,
            summary: row.try_get("summary")?,
            turn_type,
            complexity_score: row.try_get("complexity_score").ok(),
            model_used: row.try_get("model_used").ok(),
            prompt_version: row.try_get("prompt_version")?,
            generated_at,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::{ChatSessionRepository, DatabaseManager, DetectedTurnRepository};
    use crate::models::{ChatSession, DetectedTurn, Provider, SessionState};

    async fn setup_db() -> (DatabaseManager, Uuid, Uuid) {
        let db = DatabaseManager::open_in_memory().await.unwrap();
        let session_repo = ChatSessionRepository::new(&db);
        let turn_repo = DetectedTurnRepository::new(&db);

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

        // Create a detected turn
        let turn = DetectedTurn::new(session_id, 1, Utc::now());
        let turn_id = turn.id;
        turn_repo.create(&turn).await.unwrap();

        (db, session_id, turn_id)
    }

    #[tokio::test]
    async fn test_create_and_get_turn_summary() {
        let (db, _session_id, turn_id) = setup_db().await;
        let repo = TurnSummaryRepository::new(&db);

        let summary = TurnSummary::new(
            turn_id,
            "User wanted to add authentication".to_string(),
            "Created auth module".to_string(),
        )
        .with_turn_type(TurnType::Task)
        .with_complexity(0.8);

        repo.create(&summary).await.unwrap();

        let retrieved = repo.get_by_id(&summary.id).await.unwrap();
        assert!(retrieved.is_some());

        let retrieved = retrieved.unwrap();
        assert_eq!(retrieved.turn_id, turn_id);
        assert_eq!(retrieved.user_intent, "User wanted to add authentication");
        assert_eq!(retrieved.turn_type, Some(TurnType::Task));
        assert_eq!(retrieved.complexity_score, Some(0.8));
    }

    #[tokio::test]
    async fn test_get_by_turn() {
        let (db, _session_id, turn_id) = setup_db().await;
        let repo = TurnSummaryRepository::new(&db);

        let summary = TurnSummary::new(turn_id, "intent".to_string(), "action".to_string());
        repo.create(&summary).await.unwrap();

        let retrieved = repo.get_by_turn(&turn_id).await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().turn_id, turn_id);
    }

    #[tokio::test]
    async fn test_get_by_session() {
        let (db, session_id, turn_id) = setup_db().await;
        let turn_repo = DetectedTurnRepository::new(&db);
        let summary_repo = TurnSummaryRepository::new(&db);

        // Create another turn for the same session
        let turn2 = DetectedTurn::new(session_id, 2, Utc::now());
        let turn_id2 = turn2.id;
        turn_repo.create(&turn2).await.unwrap();

        // Create summaries for both turns
        let summary1 = TurnSummary::new(turn_id, "i1".to_string(), "a1".to_string());
        let summary2 = TurnSummary::new(turn_id2, "i2".to_string(), "a2".to_string());
        summary_repo.create(&summary1).await.unwrap();
        summary_repo.create(&summary2).await.unwrap();

        let summaries = summary_repo.get_by_session(&session_id).await.unwrap();
        assert_eq!(summaries.len(), 2);
    }

    #[tokio::test]
    async fn test_update() {
        let (db, _session_id, turn_id) = setup_db().await;
        let repo = TurnSummaryRepository::new(&db);

        let mut summary =
            TurnSummary::new(turn_id, "old intent".to_string(), "old action".to_string());
        repo.create(&summary).await.unwrap();

        summary.user_intent = "new intent".to_string();
        summary.turn_type = Some(TurnType::Question);

        let updated = repo.update(&summary).await.unwrap();
        assert!(updated);

        let retrieved = repo.get_by_id(&summary.id).await.unwrap().unwrap();
        assert_eq!(retrieved.user_intent, "new intent");
        assert_eq!(retrieved.turn_type, Some(TurnType::Question));
    }

    #[tokio::test]
    async fn test_get_turns_without_summaries() {
        let (db, session_id, turn_id) = setup_db().await;
        let turn_repo = DetectedTurnRepository::new(&db);
        let summary_repo = TurnSummaryRepository::new(&db);

        // Create another turn without a summary
        let turn2 = DetectedTurn::new(session_id, 2, Utc::now());
        let turn_id2 = turn2.id;
        turn_repo.create(&turn2).await.unwrap();

        // Create summary only for first turn
        let summary = TurnSummary::new(turn_id, "i".to_string(), "a".to_string());
        summary_repo.create(&summary).await.unwrap();

        let turns_without = summary_repo
            .get_turns_without_summaries(&session_id)
            .await
            .unwrap();
        assert_eq!(turns_without.len(), 1);
        assert_eq!(turns_without[0], turn_id2);
    }
}
