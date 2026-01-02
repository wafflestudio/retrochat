use anyhow::{Context, Result as AnyhowResult};
use chrono::{DateTime, Utc};
use sqlx::{Pool, Row, Sqlite};

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

    /// Create a new turn summary
    pub async fn create(&self, summary: &TurnSummary) -> AnyhowResult<String> {
        let started_at = summary.started_at.to_rfc3339();
        let ended_at = summary.ended_at.to_rfc3339();
        let generated_at = summary.generated_at.to_rfc3339();
        let turn_type = summary.turn_type.as_ref().map(|t| t.to_string());

        // Serialize JSON arrays
        let key_topics_json = summary
            .key_topics
            .as_ref()
            .map(serde_json::to_string)
            .transpose()
            .context("Failed to serialize key_topics")?;
        let decisions_made_json = summary
            .decisions_made
            .as_ref()
            .map(serde_json::to_string)
            .transpose()
            .context("Failed to serialize decisions_made")?;
        let code_concepts_json = summary
            .code_concepts
            .as_ref()
            .map(serde_json::to_string)
            .transpose()
            .context("Failed to serialize code_concepts")?;

        sqlx::query(
            r#"
            INSERT INTO turn_summaries (
                id, session_id, turn_number,
                start_sequence, end_sequence,
                user_intent, assistant_action, summary,
                turn_type, key_topics, decisions_made, code_concepts,
                started_at, ended_at,
                model_used, prompt_version, generated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&summary.id)
        .bind(&summary.session_id)
        .bind(summary.turn_number)
        .bind(summary.start_sequence)
        .bind(summary.end_sequence)
        .bind(&summary.user_intent)
        .bind(&summary.assistant_action)
        .bind(&summary.summary)
        .bind(&turn_type)
        .bind(&key_topics_json)
        .bind(&decisions_made_json)
        .bind(&code_concepts_json)
        .bind(&started_at)
        .bind(&ended_at)
        .bind(&summary.model_used)
        .bind(summary.prompt_version)
        .bind(&generated_at)
        .execute(&self.pool)
        .await
        .context("Failed to insert turn summary")?;

        Ok(summary.id.clone())
    }

    /// Get a turn summary by ID
    pub async fn get_by_id(&self, id: &str) -> AnyhowResult<Option<TurnSummary>> {
        let row = sqlx::query(
            r#"
            SELECT
                id, session_id, turn_number,
                start_sequence, end_sequence,
                user_intent, assistant_action, summary,
                turn_type, key_topics, decisions_made, code_concepts,
                started_at, ended_at,
                model_used, prompt_version, generated_at
            FROM turn_summaries
            WHERE id = ?
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .context("Failed to fetch turn summary")?;

        row.map(|r| Self::row_to_turn_summary(&r)).transpose()
    }

    /// Get all turn summaries for a session, ordered by turn number
    pub async fn get_by_session(&self, session_id: &str) -> AnyhowResult<Vec<TurnSummary>> {
        let rows = sqlx::query(
            r#"
            SELECT
                id, session_id, turn_number,
                start_sequence, end_sequence,
                user_intent, assistant_action, summary,
                turn_type, key_topics, decisions_made, code_concepts,
                started_at, ended_at,
                model_used, prompt_version, generated_at
            FROM turn_summaries
            WHERE session_id = ?
            ORDER BY turn_number ASC
            "#,
        )
        .bind(session_id)
        .fetch_all(&self.pool)
        .await
        .context("Failed to fetch turn summaries for session")?;

        rows.iter().map(Self::row_to_turn_summary).collect()
    }

    /// Get a specific turn summary by session and turn number
    pub async fn get_by_session_and_turn(
        &self,
        session_id: &str,
        turn_number: i32,
    ) -> AnyhowResult<Option<TurnSummary>> {
        let row = sqlx::query(
            r#"
            SELECT
                id, session_id, turn_number,
                start_sequence, end_sequence,
                user_intent, assistant_action, summary,
                turn_type, key_topics, decisions_made, code_concepts,
                started_at, ended_at,
                model_used, prompt_version, generated_at
            FROM turn_summaries
            WHERE session_id = ? AND turn_number = ?
            "#,
        )
        .bind(session_id)
        .bind(turn_number)
        .fetch_optional(&self.pool)
        .await
        .context("Failed to fetch turn summary")?;

        row.map(|r| Self::row_to_turn_summary(&r)).transpose()
    }

    /// Count turn summaries for a session
    pub async fn count_by_session(&self, session_id: &str) -> AnyhowResult<i64> {
        let row = sqlx::query(
            r#"
            SELECT COUNT(*) as count FROM turn_summaries
            WHERE session_id = ?
            "#,
        )
        .bind(session_id)
        .fetch_one(&self.pool)
        .await
        .context("Failed to count turn summaries")?;

        Ok(row.get::<i64, _>("count"))
    }

    /// Delete all turn summaries for a session
    pub async fn delete_by_session(&self, session_id: &str) -> AnyhowResult<u64> {
        let result = sqlx::query(
            r#"
            DELETE FROM turn_summaries
            WHERE session_id = ?
            "#,
        )
        .bind(session_id)
        .execute(&self.pool)
        .await
        .context("Failed to delete turn summaries")?;

        Ok(result.rows_affected())
    }

    /// Search turn summaries using full-text search
    pub async fn search(&self, query: &str, limit: i64) -> AnyhowResult<Vec<TurnSummary>> {
        let rows = sqlx::query(
            r#"
            SELECT
                ts.id, ts.session_id, ts.turn_number,
                ts.start_sequence, ts.end_sequence,
                ts.user_intent, ts.assistant_action, ts.summary,
                ts.turn_type, ts.key_topics, ts.decisions_made, ts.code_concepts,
                ts.started_at, ts.ended_at,
                ts.model_used, ts.prompt_version, ts.generated_at
            FROM turn_summaries ts
            JOIN turn_summaries_fts fts ON ts.rowid = fts.rowid
            WHERE turn_summaries_fts MATCH ?
            ORDER BY rank
            LIMIT ?
            "#,
        )
        .bind(query)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .context("Failed to search turn summaries")?;

        rows.iter().map(Self::row_to_turn_summary).collect()
    }

    /// Convert a database row to TurnSummary
    fn row_to_turn_summary(row: &sqlx::sqlite::SqliteRow) -> AnyhowResult<TurnSummary> {
        let started_at_str: String = row.get("started_at");
        let ended_at_str: String = row.get("ended_at");
        let generated_at_str: String = row.get("generated_at");

        let started_at = DateTime::parse_from_rfc3339(&started_at_str)?.with_timezone(&Utc);
        let ended_at = DateTime::parse_from_rfc3339(&ended_at_str)?.with_timezone(&Utc);
        let generated_at = DateTime::parse_from_rfc3339(&generated_at_str)?.with_timezone(&Utc);

        let turn_type_str: Option<String> = row.get("turn_type");
        let turn_type = turn_type_str
            .map(|t| t.parse::<TurnType>())
            .transpose()
            .ok()
            .flatten();

        let key_topics_json: Option<String> = row.get("key_topics");
        let key_topics: Option<Vec<String>> = key_topics_json
            .map(|t| serde_json::from_str(&t))
            .transpose()
            .context("Failed to deserialize key_topics")?;

        let decisions_made_json: Option<String> = row.get("decisions_made");
        let decisions_made: Option<Vec<String>> = decisions_made_json
            .map(|d| serde_json::from_str(&d))
            .transpose()
            .context("Failed to deserialize decisions_made")?;

        let code_concepts_json: Option<String> = row.get("code_concepts");
        let code_concepts: Option<Vec<String>> = code_concepts_json
            .map(|c| serde_json::from_str(&c))
            .transpose()
            .context("Failed to deserialize code_concepts")?;

        Ok(TurnSummary {
            id: row.get("id"),
            session_id: row.get("session_id"),
            turn_number: row.get("turn_number"),
            start_sequence: row.get("start_sequence"),
            end_sequence: row.get("end_sequence"),
            user_intent: row.get("user_intent"),
            assistant_action: row.get("assistant_action"),
            summary: row.get("summary"),
            turn_type,
            key_topics,
            decisions_made,
            code_concepts,
            started_at,
            ended_at,
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
    use crate::models::TurnType;

    #[tokio::test]
    async fn test_create_and_get_turn_summary() {
        let db = DatabaseManager::open_in_memory().await.unwrap();
        let repo = TurnSummaryRepository::new(&db);

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

        let now = Utc::now();
        let summary = TurnSummary::new(
            "session-1".to_string(),
            0,
            1,
            5,
            "Add authentication".to_string(),
            "Created JWT module".to_string(),
            "User wanted auth, Claude created JWT".to_string(),
            now,
            now,
        )
        .with_turn_type(TurnType::Task)
        .with_key_topics(vec!["auth".to_string(), "jwt".to_string()]);

        let id = repo.create(&summary).await.unwrap();
        assert!(!id.is_empty());

        let fetched = repo.get_by_id(&id).await.unwrap().unwrap();
        assert_eq!(fetched.session_id, "session-1");
        assert_eq!(fetched.turn_number, 0);
        assert_eq!(fetched.user_intent, "Add authentication");
        assert_eq!(fetched.turn_type, Some(TurnType::Task));
        assert_eq!(
            fetched.key_topics,
            Some(vec!["auth".to_string(), "jwt".to_string()])
        );
    }

    #[tokio::test]
    async fn test_get_by_session() {
        let db = DatabaseManager::open_in_memory().await.unwrap();
        let repo = TurnSummaryRepository::new(&db);

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

        let now = Utc::now();

        // Create multiple turns
        for i in 0..3 {
            let summary = TurnSummary::new(
                "session-2".to_string(),
                i,
                i * 3 + 1,
                (i + 1) * 3,
                format!("Intent {i}"),
                format!("Action {i}"),
                format!("Summary {i}"),
                now,
                now,
            );
            repo.create(&summary).await.unwrap();
        }

        let turns = repo.get_by_session("session-2").await.unwrap();
        assert_eq!(turns.len(), 3);
        assert_eq!(turns[0].turn_number, 0);
        assert_eq!(turns[1].turn_number, 1);
        assert_eq!(turns[2].turn_number, 2);
    }

    #[tokio::test]
    async fn test_count_by_session() {
        let db = DatabaseManager::open_in_memory().await.unwrap();
        let repo = TurnSummaryRepository::new(&db);

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

        let now = Utc::now();
        let summary = TurnSummary::new(
            "session-3".to_string(),
            0,
            1,
            5,
            "intent".to_string(),
            "action".to_string(),
            "summary".to_string(),
            now,
            now,
        );
        repo.create(&summary).await.unwrap();

        let count = repo.count_by_session("session-3").await.unwrap();
        assert_eq!(count, 1);
    }

    #[tokio::test]
    async fn test_delete_by_session() {
        let db = DatabaseManager::open_in_memory().await.unwrap();
        let repo = TurnSummaryRepository::new(&db);

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

        let now = Utc::now();
        let summary = TurnSummary::new(
            "session-4".to_string(),
            0,
            1,
            5,
            "intent".to_string(),
            "action".to_string(),
            "summary".to_string(),
            now,
            now,
        );
        repo.create(&summary).await.unwrap();

        let deleted = repo.delete_by_session("session-4").await.unwrap();
        assert_eq!(deleted, 1);

        let count = repo.count_by_session("session-4").await.unwrap();
        assert_eq!(count, 0);
    }
}
