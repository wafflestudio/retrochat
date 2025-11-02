use anyhow::{Context, Result as AnyhowResult};
use chrono::{DateTime, Duration, Utc};
use sqlx::{Pool, Row, Sqlite};
use std::collections::HashMap;

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

    pub async fn get_daily_usage_stats(
        &self,
        date: DateTime<Utc>,
    ) -> AnyhowResult<DailyUsageStats> {
        let start_of_day = date.date_naive().and_hms_opt(0, 0, 0).unwrap().and_utc();
        let end_of_day = start_of_day + Duration::days(1);

        // Get total sessions
        let total_sessions: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM chat_sessions WHERE created_at >= ? AND created_at < ?",
        )
        .bind(start_of_day.to_rfc3339())
        .bind(end_of_day.to_rfc3339())
        .fetch_one(&self.pool)
        .await
        .context("Failed to get total sessions")?;

        // Get total messages
        let total_messages: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*) FROM messages m
            JOIN chat_sessions cs ON m.session_id = cs.id
            WHERE cs.created_at >= ? AND cs.created_at < ?
            "#,
        )
        .bind(start_of_day.to_rfc3339())
        .bind(end_of_day.to_rfc3339())
        .fetch_one(&self.pool)
        .await
        .context("Failed to get total messages")?;

        // Get total tokens
        let total_tokens: i64 = sqlx::query_scalar(
            "SELECT COALESCE(SUM(token_count), 0) FROM chat_sessions WHERE created_at >= ? AND created_at < ?"
        )
        .bind(start_of_day.to_rfc3339())
        .bind(end_of_day.to_rfc3339())
        .fetch_one(&self.pool)
        .await
        .context("Failed to get total tokens")?;

        // Get provider usage
        let provider_rows = sqlx::query(
            "SELECT provider, COUNT(*) as count FROM chat_sessions WHERE created_at >= ? AND created_at < ? GROUP BY provider"
        )
        .bind(start_of_day.to_rfc3339())
        .bind(end_of_day.to_rfc3339())
        .fetch_all(&self.pool)
        .await
        .context("Failed to get provider usage")?;

        let mut provider_usage = HashMap::new();
        for row in provider_rows {
            let provider: String = row.try_get("provider")?;
            let count: i64 = row.try_get("count")?;
            provider_usage.insert(provider, count as u32);
        }

        // Get hourly distribution
        let hourly_rows = sqlx::query(
            r#"
            SELECT strftime('%H', created_at) as hour, COUNT(*) as count
            FROM chat_sessions
            WHERE created_at >= ? AND created_at < ?
            GROUP BY hour
            ORDER BY hour
            "#,
        )
        .bind(start_of_day.to_rfc3339())
        .bind(end_of_day.to_rfc3339())
        .fetch_all(&self.pool)
        .await
        .context("Failed to get hourly distribution")?;

        let mut hourly_distribution = vec![0u32; 24];
        for row in hourly_rows {
            let hour_str: String = row.try_get("hour")?;
            let count: i64 = row.try_get("count")?;
            if let Ok(hour) = hour_str.parse::<u8>() {
                if (hour as usize) < 24 {
                    hourly_distribution[hour as usize] = count as u32;
                }
            }
        }

        Ok(DailyUsageStats {
            date,
            total_sessions: total_sessions as u32,
            total_messages: total_messages as u32,
            total_tokens: total_tokens as u64,
            provider_usage,
            hourly_distribution,
            avg_session_length: if total_sessions > 0 {
                total_messages as f64 / total_sessions as f64
            } else {
                0.0
            },
            avg_tokens_per_session: if total_sessions > 0 {
                total_tokens as f64 / total_sessions as f64
            } else {
                0.0
            },
        })
    }

    pub async fn get_provider_usage_trends(&self, days: u32) -> AnyhowResult<Vec<ProviderTrend>> {
        let end_date = Utc::now();
        let start_date = end_date - Duration::days(days as i64);

        let trend_rows = sqlx::query(
            r#"
            SELECT provider,
                   DATE(created_at) as date,
                   COUNT(*) as session_count,
                   COALESCE(SUM(token_count), 0) as token_count
            FROM chat_sessions
            WHERE created_at >= ? AND created_at <= ?
            GROUP BY provider, DATE(created_at)
            ORDER BY provider, date
            "#,
        )
        .bind(start_date.to_rfc3339())
        .bind(end_date.to_rfc3339())
        .fetch_all(&self.pool)
        .await
        .context("Failed to get provider usage trends")?;

        let mut provider_data: HashMap<String, Vec<DailyPoint>> = HashMap::new();
        for row in trend_rows {
            let provider: String = row.try_get("provider")?;
            let date_str: String = row.try_get("date")?;
            let session_count: i64 = row.try_get("session_count")?;
            let token_count: i64 = row.try_get("token_count")?;

            let date =
                DateTime::parse_from_str(&format!("{date_str}T00:00:00Z"), "%Y-%m-%dT%H:%M:%SZ")
                    .context("Failed to parse date")?
                    .with_timezone(&Utc);

            provider_data.entry(provider).or_default().push(DailyPoint {
                date,
                session_count: session_count as u32,
                token_count: token_count as u64,
            });
        }

        let mut trends = Vec::new();
        for (provider, data) in provider_data {
            let total_sessions: u32 = data.iter().map(|d| d.session_count).sum();
            let total_tokens: u64 = data.iter().map(|d| d.token_count).sum();

            trends.push(ProviderTrend {
                provider,
                total_sessions,
                total_tokens,
                daily_data: data,
            });
        }

        Ok(trends)
    }

    pub async fn get_session_length_distribution(&self) -> AnyhowResult<SessionLengthDistribution> {
        let dist_rows = sqlx::query(
            r#"
            SELECT
                CASE
                    WHEN message_count <= 5 THEN 'short'
                    WHEN message_count <= 20 THEN 'medium'
                    WHEN message_count <= 50 THEN 'long'
                    ELSE 'very_long'
                END as length_category,
                COUNT(*) as count
            FROM chat_sessions
            GROUP BY length_category
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .context("Failed to get session length distribution")?;

        let mut short = 0;
        let mut medium = 0;
        let mut long = 0;
        let mut very_long = 0;

        for row in dist_rows {
            let category: String = row.try_get("length_category")?;
            let count: i64 = row.try_get("count")?;

            match category.as_str() {
                "short" => short = count as u32,
                "medium" => medium = count as u32,
                "long" => long = count as u32,
                "very_long" => very_long = count as u32,
                _ => {}
            }
        }

        let total = short + medium + long + very_long;
        let total_f64 = total as f64;

        Ok(SessionLengthDistribution {
            short_sessions: short,
            medium_sessions: medium,
            long_sessions: long,
            very_long_sessions: very_long,
            short_percentage: if total > 0 {
                short as f64 / total_f64 * 100.0
            } else {
                0.0
            },
            medium_percentage: if total > 0 {
                medium as f64 / total_f64 * 100.0
            } else {
                0.0
            },
            long_percentage: if total > 0 {
                long as f64 / total_f64 * 100.0
            } else {
                0.0
            },
            very_long_percentage: if total > 0 {
                very_long as f64 / total_f64 * 100.0
            } else {
                0.0
            },
        })
    }

    pub async fn get_hourly_activity(&self, days: u32) -> AnyhowResult<Vec<HourlyActivity>> {
        let end_date = Utc::now();
        let start_date = end_date - Duration::days(days as i64);

        let hourly_rows = sqlx::query(
            r#"
            SELECT strftime('%H', created_at) as hour,
                   COUNT(*) as session_count,
                   COALESCE(SUM(token_count), 0) as token_count
            FROM chat_sessions
            WHERE created_at >= ? AND created_at <= ?
            GROUP BY hour
            ORDER BY hour
            "#,
        )
        .bind(start_date.to_rfc3339())
        .bind(end_date.to_rfc3339())
        .fetch_all(&self.pool)
        .await
        .context("Failed to get hourly activity")?;

        let mut hourly_activity = Vec::new();
        for row in hourly_rows {
            let hour_str: String = row.try_get("hour")?;
            let session_count: i64 = row.try_get("session_count")?;
            let token_count: i64 = row.try_get("token_count")?;

            if let Ok(hour) = hour_str.parse::<u8>() {
                hourly_activity.push(HourlyActivity {
                    hour,
                    session_count: session_count as u32,
                    token_count: token_count as u64,
                });
            }
        }

        Ok(hourly_activity)
    }

    pub async fn generate_insights(&self, days: u32) -> AnyhowResult<Vec<String>> {
        let end_date = Utc::now();
        let start_date = end_date - Duration::days(days as i64);

        // Get overall stats
        let total_sessions: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM chat_sessions WHERE created_at >= ? AND created_at <= ?",
        )
        .bind(start_date.to_rfc3339())
        .bind(end_date.to_rfc3339())
        .fetch_one(&self.pool)
        .await
        .context("Failed to get total sessions")?;

        let total_tokens: i64 = sqlx::query_scalar(
            "SELECT COALESCE(SUM(token_count), 0) FROM chat_sessions WHERE created_at >= ? AND created_at <= ?"
        )
        .bind(start_date.to_rfc3339())
        .bind(end_date.to_rfc3339())
        .fetch_one(&self.pool)
        .await
        .context("Failed to get total tokens")?;

        // Get most used provider
        let most_used_provider: Option<String> = sqlx::query_scalar(
            r#"
            SELECT provider FROM chat_sessions 
            WHERE created_at >= ? AND created_at <= ?
            GROUP BY provider 
            ORDER BY COUNT(*) DESC 
            LIMIT 1
            "#,
        )
        .bind(start_date.to_rfc3339())
        .bind(end_date.to_rfc3339())
        .fetch_optional(&self.pool)
        .await
        .context("Failed to get most used provider")?;

        let mut insights = Vec::new();

        if total_sessions > 0 {
            insights.push(format!(
                "📊 Usage Summary: {total_sessions} sessions with {total_tokens} tokens over the last {days} days"
            ));

            if let Some(provider) = most_used_provider {
                insights.push(format!("🎯 Most used provider: {provider}"));
            }

            let avg_tokens = total_tokens as f64 / total_sessions as f64;
            insights.push(format!("📈 Average tokens per session: {avg_tokens:.1}"));

            // Get session length distribution
            let distribution = self.get_session_length_distribution().await?;
            if distribution.short_sessions > distribution.medium_sessions {
                insights.push("💡 Most sessions are short (≤5 messages) - consider longer conversations for better context".to_string());
            } else if distribution.very_long_sessions > 0 {
                insights.push(
                    "🔥 You have some very long sessions (>50 messages) - great for complex tasks!"
                        .to_string(),
                );
            }
        } else {
            insights.push(
                "📝 No activity in the selected time period. Start chatting to see insights!"
                    .to_string(),
            );
        }

        Ok(insights)
    }

    pub async fn get_total_stats(&self) -> AnyhowResult<(u32, u32, u64)> {
        let total_sessions: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM chat_sessions")
            .fetch_one(&self.pool)
            .await
            .context("Failed to get total sessions")?;

        let total_messages: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM messages")
            .fetch_one(&self.pool)
            .await
            .context("Failed to get total messages")?;

        let total_tokens: i64 =
            sqlx::query_scalar("SELECT COALESCE(SUM(token_count), 0) FROM chat_sessions")
                .fetch_one(&self.pool)
                .await
                .context("Failed to get total tokens")?;

        Ok((
            total_sessions as u32,
            total_messages as u32,
            total_tokens as u64,
        ))
    }

    pub async fn save_analytics(&self, analytics: &Analytics) -> AnyhowResult<String> {
        let generated_at = analytics.generated_at.to_rfc3339();

        // Serialize JSON fields
        let scores_json =
            serde_json::to_string(&analytics.scores).context("Failed to serialize scores")?;
        let metrics_json =
            serde_json::to_string(&analytics.metrics).context("Failed to serialize metrics")?;
        let quantitative_input_json = serde_json::to_string(&analytics.quantitative_input)
            .context("Failed to serialize quantitative_input")?;
        let qualitative_input_json = serde_json::to_string(&analytics.qualitative_input)
            .context("Failed to serialize qualitative_input")?;
        let qualitative_output_json = serde_json::to_string(&analytics.qualitative_output)
            .context("Failed to serialize qualitative_output")?;
        let processed_output_json = serde_json::to_string(&analytics.processed_output)
            .context("Failed to serialize processed_output")?;

        sqlx::query!(
            r#"
            INSERT INTO analytics (
                id, analytics_request_id, session_id, generated_at,
                scores_json, metrics_json,
                quantitative_input_json, qualitative_input_json,
                qualitative_output_json, processed_output_json,
                model_used, analysis_duration_ms
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
            analytics.id,
            analytics.analytics_request_id,
            analytics.session_id,
            generated_at,
            scores_json,
            metrics_json,
            quantitative_input_json,
            qualitative_input_json,
            qualitative_output_json,
            processed_output_json,
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
                scores_json, metrics_json,
                quantitative_input_json, qualitative_input_json,
                qualitative_output_json, processed_output_json,
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
            let scores: crate::models::Scores =
                serde_json::from_str(&row.scores_json).context("Failed to deserialize scores")?;
            let metrics: crate::models::Metrics =
                serde_json::from_str(&row.metrics_json).context("Failed to deserialize metrics")?;
            let quantitative_input: crate::services::analytics::QuantitativeInput =
                serde_json::from_str(&row.quantitative_input_json)
                    .context("Failed to deserialize quantitative_input")?;
            let qualitative_input: crate::services::analytics::QualitativeInput =
                serde_json::from_str(&row.qualitative_input_json)
                    .context("Failed to deserialize qualitative_input")?;
            let qualitative_output: crate::services::analytics::QualitativeOutput =
                serde_json::from_str(&row.qualitative_output_json)
                    .context("Failed to deserialize qualitative_output")?;
            let processed_output: crate::services::analytics::ProcessedQuantitativeOutput =
                serde_json::from_str(&row.processed_output_json)
                    .context("Failed to deserialize processed_output")?;

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
                scores,
                metrics,
                quantitative_input,
                qualitative_input,
                qualitative_output,
                processed_output,
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
                scores_json, metrics_json,
                quantitative_input_json, qualitative_input_json,
                qualitative_output_json, processed_output_json,
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
            let scores: crate::models::Scores =
                serde_json::from_str(&row.scores_json).context("Failed to deserialize scores")?;
            let metrics: crate::models::Metrics =
                serde_json::from_str(&row.metrics_json).context("Failed to deserialize metrics")?;
            let quantitative_input: crate::services::analytics::QuantitativeInput =
                serde_json::from_str(&row.quantitative_input_json)
                    .context("Failed to deserialize quantitative_input")?;
            let qualitative_input: crate::services::analytics::QualitativeInput =
                serde_json::from_str(&row.qualitative_input_json)
                    .context("Failed to deserialize qualitative_input")?;
            let qualitative_output: crate::services::analytics::QualitativeOutput =
                serde_json::from_str(&row.qualitative_output_json)
                    .context("Failed to deserialize qualitative_output")?;
            let processed_output: crate::services::analytics::ProcessedQuantitativeOutput =
                serde_json::from_str(&row.processed_output_json)
                    .context("Failed to deserialize processed_output")?;

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
                scores,
                metrics,
                quantitative_input,
                qualitative_input,
                qualitative_output,
                processed_output,
                model_used: row.model_used,
                analysis_duration_ms: row.analysis_duration_ms,
            }))
        } else {
            Ok(None)
        }
    }
}

// Data structures (same as original)
#[derive(Debug)]
pub struct DailyUsageStats {
    pub date: DateTime<Utc>,
    pub total_sessions: u32,
    pub total_messages: u32,
    pub total_tokens: u64,
    pub provider_usage: HashMap<String, u32>,
    pub hourly_distribution: Vec<u32>, // 24 elements for hours 0-23
    pub avg_session_length: f64,
    pub avg_tokens_per_session: f64,
}

#[derive(Debug)]
pub struct ProviderTrend {
    pub provider: String,
    pub total_sessions: u32,
    pub total_tokens: u64,
    pub daily_data: Vec<DailyPoint>,
}

#[derive(Debug)]
pub struct DailyPoint {
    pub date: DateTime<Utc>,
    pub session_count: u32,
    pub token_count: u64,
}

#[derive(Debug)]
pub struct SessionLengthDistribution {
    pub short_sessions: u32,     // <= 5 messages
    pub medium_sessions: u32,    // 6-20 messages
    pub long_sessions: u32,      // 21-50 messages
    pub very_long_sessions: u32, // > 50 messages
    pub short_percentage: f64,
    pub medium_percentage: f64,
    pub long_percentage: f64,
    pub very_long_percentage: f64,
}

#[derive(Debug)]
pub struct HourlyActivity {
    pub hour: u8, // 0-23
    pub session_count: u32,
    pub token_count: u64,
}
