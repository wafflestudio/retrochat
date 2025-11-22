use anyhow::{Context, Result as AnyhowResult};
use chrono::{DateTime, Utc};
use sqlx::{Pool, Sqlite};

use super::connection::DatabaseManager;
use crate::models::Analytics;

pub struct AnalyticsRepository {
    pool: Pool<Sqlite>,
}

impl AnalyticsRepository {
    pub fn new(db: &DatabaseManager) -> Self {
        Self {
            pool: db.pool().clone(),
        }
    }

    pub async fn save_analytics(&self, analytics: &Analytics) -> AnyhowResult<String> {
        let generated_at = analytics.generated_at.to_rfc3339();

        // Serialize JSON fields
        let metrics_json =
            serde_json::to_string(&analytics.metrics).context("Failed to serialize metrics")?;
        let qualitative_output_json = serde_json::to_string(&analytics.qualitative_output)
            .context("Failed to serialize qualitative_output")?;
        let ai_quantitative_output_json = serde_json::to_string(&analytics.ai_quantitative_output)
            .context("Failed to serialize ai_quantitative_output")?;

        sqlx::query!(
            r#"
            INSERT INTO analytics (
                id, analytics_request_id, session_id, generated_at,
                metrics_json,
                qualitative_output_json,
                ai_quantitative_output_json,
                model_used, analysis_duration_ms
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
            analytics.id,
            analytics.analytics_request_id,
            analytics.session_id,
            generated_at,
            metrics_json,
            qualitative_output_json,
            ai_quantitative_output_json,
            analytics.model_used,
            analytics.analysis_duration_ms
        )
        .execute(&self.pool)
        .await
        .context("Failed to insert analytics")?;

        Ok(analytics.id.clone())
    }

    pub async fn get_analytics_by_id(&self, id: &str) -> AnyhowResult<Option<Analytics>> {
        let row = sqlx::query!(
            r#"
            SELECT
                id, analytics_request_id, session_id, generated_at,
                metrics_json,
                qualitative_output_json,
                ai_quantitative_output_json,
                model_used, analysis_duration_ms
            FROM analytics
            WHERE id = ?
            "#,
            id
        )
        .fetch_optional(&self.pool)
        .await
        .context("Failed to fetch analytics")?;

        if let Some(row) = row {
            let generated_at = DateTime::parse_from_rfc3339(&row.generated_at)?.with_timezone(&Utc);

            // Deserialize JSON fields
            let metrics: crate::models::Metrics =
                serde_json::from_str(&row.metrics_json).context("Failed to deserialize metrics")?;
            let qualitative_output: crate::services::analytics::AIQualitativeOutput =
                serde_json::from_str(&row.qualitative_output_json)
                    .context("Failed to deserialize qualitative_output")?;
            let ai_quantitative_output: crate::services::analytics::AIQuantitativeOutput =
                serde_json::from_str(&row.ai_quantitative_output_json)
                    .context("Failed to deserialize ai_quantitative_output")?;

            // Get session_id from row, or fetch from analytics_requests as fallback
            let session_id = if let Some(sid) = row.session_id {
                sid
            } else {
                sqlx::query_scalar!(
                    "SELECT session_id FROM analytics_requests WHERE id = ?",
                    row.analytics_request_id
                )
                .fetch_optional(&self.pool)
                .await?
                .unwrap_or_else(|| "unknown".to_string())
            };

            Ok(Some(Analytics {
                id: row.id.unwrap_or_default(),
                analytics_request_id: row.analytics_request_id,
                session_id,
                generated_at,
                metrics,
                qualitative_output,
                ai_quantitative_output,
                model_used: row.model_used,
                analysis_duration_ms: row.analysis_duration_ms,
            }))
        } else {
            Ok(None)
        }
    }

    pub async fn get_analytics_by_request_id(
        &self,
        analytics_request_id: &str,
    ) -> AnyhowResult<Option<Analytics>> {
        let row = sqlx::query!(
            r#"
            SELECT
                id, analytics_request_id, session_id, generated_at,
                metrics_json,
                qualitative_output_json,
                ai_quantitative_output_json,
                model_used, analysis_duration_ms
            FROM analytics
            WHERE analytics_request_id = ?
            ORDER BY generated_at DESC
            LIMIT 1
            "#,
            analytics_request_id
        )
        .fetch_optional(&self.pool)
        .await
        .context("Failed to fetch analytics by request_id")?;

        if let Some(row) = row {
            let generated_at = DateTime::parse_from_rfc3339(&row.generated_at)?.with_timezone(&Utc);

            // Deserialize JSON fields
            let metrics: crate::models::Metrics =
                serde_json::from_str(&row.metrics_json).context("Failed to deserialize metrics")?;
            let qualitative_output: crate::services::analytics::AIQualitativeOutput =
                serde_json::from_str(&row.qualitative_output_json)
                    .context("Failed to deserialize qualitative_output")?;
            let ai_quantitative_output: crate::services::analytics::AIQuantitativeOutput =
                serde_json::from_str(&row.ai_quantitative_output_json)
                    .context("Failed to deserialize ai_quantitative_output")?;

            // Get session_id from row, or fetch from analytics_requests as fallback
            let session_id = if let Some(sid) = row.session_id {
                sid
            } else {
                sqlx::query_scalar!(
                    "SELECT session_id FROM analytics_requests WHERE id = ?",
                    analytics_request_id
                )
                .fetch_optional(&self.pool)
                .await?
                .unwrap_or_else(|| "unknown".to_string())
            };

            Ok(Some(Analytics {
                id: row.id.unwrap_or_default(),
                analytics_request_id: row.analytics_request_id,
                session_id,
                generated_at,
                metrics,
                qualitative_output,
                ai_quantitative_output,
                model_used: row.model_used,
                analysis_duration_ms: row.analysis_duration_ms,
            }))
        } else {
            Ok(None)
        }
    }
}
