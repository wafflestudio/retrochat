use chrono::{DateTime, Utc};
use std::sync::Arc;

use crate::database::DatabaseManager;
use crate::models::Retrospection;
use crate::services::analytics::models::{
    ProcessedQuantitativeOutput, QualitativeInput, QualitativeOutput, QuantitativeInput,
};

// Macro to convert sqlx query result row to Retrospection
// The sqlx::query! macro generates anonymous structs with public fields
macro_rules! row_to_retrospection {
    ($row:expr) => {{
        let generated_at = DateTime::parse_from_rfc3339(&$row.generated_at)?.with_timezone(&Utc);

        let quantitative_input: QuantitativeInput =
            serde_json::from_str(&$row.quantitative_input_json)?;
        let qualitative_input: QualitativeInput =
            serde_json::from_str(&$row.qualitative_input_json)?;
        let qualitative_output: QualitativeOutput =
            serde_json::from_str(&$row.qualitative_output_json)?;
        let processed_output: ProcessedQuantitativeOutput =
            serde_json::from_str(&$row.processed_output_json)?;

        Retrospection {
            id: $row.id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string()),
            retrospect_request_id: $row.retrospect_request_id,
            generated_at,
            overall_score: $row.overall_score,
            code_quality_score: $row.code_quality_score,
            productivity_score: $row.productivity_score,
            efficiency_score: $row.efficiency_score,
            collaboration_score: $row.collaboration_score,
            learning_score: $row.learning_score,
            total_files_modified: $row.total_files_modified as i32,
            total_files_read: $row.total_files_read as i32,
            lines_added: $row.lines_added as i32,
            lines_removed: $row.lines_removed as i32,
            total_tokens_used: $row.total_tokens_used as i32,
            session_duration_minutes: $row.session_duration_minutes,
            quantitative_input,
            qualitative_input,
            qualitative_output,
            processed_output,
            model_used: $row.model_used,
            analysis_duration_ms: $row.analysis_duration_ms,
        }
    }};
}

#[derive(Clone)]
pub struct RetrospectionRepository {
    db_manager: Arc<DatabaseManager>,
}

impl RetrospectionRepository {
    pub fn new(db_manager: Arc<DatabaseManager>) -> Self {
        Self { db_manager }
    }

    pub async fn create(
        &self,
        retrospection: &Retrospection,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let pool = self.db_manager.pool();

        let generated_at_str = retrospection.generated_at.to_rfc3339();

        // Serialize JSON fields
        let quantitative_input_json = serde_json::to_string(&retrospection.quantitative_input)?;
        let qualitative_input_json = serde_json::to_string(&retrospection.qualitative_input)?;
        let qualitative_output_json = serde_json::to_string(&retrospection.qualitative_output)?;
        let processed_output_json = serde_json::to_string(&retrospection.processed_output)?;

        sqlx::query!(
            r#"
            INSERT INTO retrospections (
                id, retrospect_request_id, generated_at,
                overall_score, code_quality_score, productivity_score,
                efficiency_score, collaboration_score, learning_score,
                total_files_modified, total_files_read,
                lines_added, lines_removed,
                total_tokens_used, session_duration_minutes,
                quantitative_input_json, qualitative_input_json,
                qualitative_output_json, processed_output_json,
                model_used, analysis_duration_ms
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
            retrospection.id,
            retrospection.retrospect_request_id,
            generated_at_str,
            retrospection.overall_score,
            retrospection.code_quality_score,
            retrospection.productivity_score,
            retrospection.efficiency_score,
            retrospection.collaboration_score,
            retrospection.learning_score,
            retrospection.total_files_modified,
            retrospection.total_files_read,
            retrospection.lines_added,
            retrospection.lines_removed,
            retrospection.total_tokens_used,
            retrospection.session_duration_minutes,
            quantitative_input_json,
            qualitative_input_json,
            qualitative_output_json,
            processed_output_json,
            retrospection.model_used,
            retrospection.analysis_duration_ms
        )
        .execute(pool)
        .await?;

        Ok(())
    }

    pub async fn find_by_id(
        &self,
        id: &str,
    ) -> Result<Option<Retrospection>, Box<dyn std::error::Error + Send + Sync>> {
        let pool = self.db_manager.pool();

        let row = sqlx::query!(
            r#"
            SELECT
                id, retrospect_request_id, generated_at,
                overall_score, code_quality_score, productivity_score,
                efficiency_score, collaboration_score, learning_score,
                total_files_modified, total_files_read,
                lines_added, lines_removed,
                total_tokens_used, session_duration_minutes,
                quantitative_input_json, qualitative_input_json,
                qualitative_output_json, processed_output_json,
                model_used, analysis_duration_ms
            FROM retrospections
            WHERE id = ?
            "#,
            id
        )
        .fetch_optional(pool)
        .await?;

        if let Some(row) = row {
            Ok(Some(row_to_retrospection!(row)))
        } else {
            Ok(None)
        }
    }

    pub async fn find_by_request_id(
        &self,
        request_id: &str,
    ) -> Result<Vec<Retrospection>, Box<dyn std::error::Error + Send + Sync>> {
        let pool = self.db_manager.pool();

        let rows = sqlx::query!(
            r#"
            SELECT
                id, retrospect_request_id, generated_at,
                overall_score, code_quality_score, productivity_score,
                efficiency_score, collaboration_score, learning_score,
                total_files_modified, total_files_read,
                lines_added, lines_removed,
                total_tokens_used, session_duration_minutes,
                quantitative_input_json, qualitative_input_json,
                qualitative_output_json, processed_output_json,
                model_used, analysis_duration_ms
            FROM retrospections
            WHERE retrospect_request_id = ?
            ORDER BY generated_at DESC
            "#,
            request_id
        )
        .fetch_all(pool)
        .await?;

        let mut retrospections = Vec::new();
        for row in rows {
            retrospections.push(row_to_retrospection!(row));
        }

        Ok(retrospections)
    }

    pub async fn get_by_session_id(
        &self,
        session_id: &str,
    ) -> Result<Vec<Retrospection>, Box<dyn std::error::Error + Send + Sync>> {
        let pool = self.db_manager.pool();

        let rows = sqlx::query!(
            r#"
            SELECT
                r.id, r.retrospect_request_id, r.generated_at,
                r.overall_score, r.code_quality_score, r.productivity_score,
                r.efficiency_score, r.collaboration_score, r.learning_score,
                r.total_files_modified, r.total_files_read,
                r.lines_added, r.lines_removed,
                r.total_tokens_used, r.session_duration_minutes,
                r.quantitative_input_json, r.qualitative_input_json,
                r.qualitative_output_json, r.processed_output_json,
                r.model_used, r.analysis_duration_ms
            FROM retrospections r
            JOIN retrospect_requests rr ON r.retrospect_request_id = rr.id
            WHERE rr.session_id = ?
            ORDER BY r.generated_at DESC
            "#,
            session_id
        )
        .fetch_all(pool)
        .await?;

        let mut retrospections = Vec::new();
        for row in rows {
            retrospections.push(row_to_retrospection!(row));
        }

        Ok(retrospections)
    }

    pub async fn find_recent(
        &self,
        limit: Option<usize>,
    ) -> Result<Vec<Retrospection>, Box<dyn std::error::Error + Send + Sync>> {
        let pool = self.db_manager.pool();

        let limit = limit.unwrap_or(10) as i64;

        let rows = sqlx::query!(
            r#"
            SELECT
                id, retrospect_request_id, generated_at,
                overall_score, code_quality_score, productivity_score,
                efficiency_score, collaboration_score, learning_score,
                total_files_modified, total_files_read,
                lines_added, lines_removed,
                total_tokens_used, session_duration_minutes,
                quantitative_input_json, qualitative_input_json,
                qualitative_output_json, processed_output_json,
                model_used, analysis_duration_ms
            FROM retrospections
            ORDER BY generated_at DESC
            LIMIT ?
            "#,
            limit
        )
        .fetch_all(pool)
        .await?;

        let mut retrospections = Vec::new();
        for row in rows {
            retrospections.push(row_to_retrospection!(row));
        }

        Ok(retrospections)
    }

    pub async fn find_since(
        &self,
        since: DateTime<Utc>,
    ) -> Result<Vec<Retrospection>, Box<dyn std::error::Error + Send + Sync>> {
        let pool = self.db_manager.pool();

        let since_str = since.to_rfc3339();
        let rows = sqlx::query!(
            r#"
            SELECT
                id, retrospect_request_id, generated_at,
                overall_score, code_quality_score, productivity_score,
                efficiency_score, collaboration_score, learning_score,
                total_files_modified, total_files_read,
                lines_added, lines_removed,
                total_tokens_used, session_duration_minutes,
                quantitative_input_json, qualitative_input_json,
                qualitative_output_json, processed_output_json,
                model_used, analysis_duration_ms
            FROM retrospections
            WHERE generated_at >= ?
            ORDER BY generated_at DESC
            "#,
            since_str
        )
        .fetch_all(pool)
        .await?;

        let mut retrospections = Vec::new();
        for row in rows {
            retrospections.push(row_to_retrospection!(row));
        }

        Ok(retrospections)
    }

    pub async fn delete_by_id(
        &self,
        id: &str,
    ) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        let pool = self.db_manager.pool();

        let result = sqlx::query!("DELETE FROM retrospections WHERE id = ?", id)
            .execute(pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }

    pub async fn delete_by_request_id(
        &self,
        request_id: &str,
    ) -> Result<u64, Box<dyn std::error::Error + Send + Sync>> {
        let pool = self.db_manager.pool();

        let result = sqlx::query!(
            "DELETE FROM retrospections WHERE retrospect_request_id = ?",
            request_id
        )
        .execute(pool)
        .await?;

        Ok(result.rows_affected())
    }

    pub async fn delete_before(
        &self,
        before: DateTime<Utc>,
    ) -> Result<u64, Box<dyn std::error::Error + Send + Sync>> {
        let pool = self.db_manager.pool();

        let before_str = before.to_rfc3339();
        let result = sqlx::query!(
            "DELETE FROM retrospections WHERE generated_at < ?",
            before_str
        )
        .execute(pool)
        .await?;

        Ok(result.rows_affected())
    }

    pub async fn count(&self) -> Result<u64, Box<dyn std::error::Error + Send + Sync>> {
        let pool = self.db_manager.pool();

        let row = sqlx::query!("SELECT COUNT(*) as count FROM retrospections")
            .fetch_one(pool)
            .await?;

        Ok(row.count as u64)
    }

    pub async fn count_by_request_id(
        &self,
        request_id: &str,
    ) -> Result<u64, Box<dyn std::error::Error + Send + Sync>> {
        let pool = self.db_manager.pool();

        let row = sqlx::query!(
            "SELECT COUNT(*) as count FROM retrospections WHERE retrospect_request_id = ?",
            request_id
        )
        .fetch_one(pool)
        .await?;

        Ok(row.count as u64)
    }
}
