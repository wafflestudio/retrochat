use anyhow::Result as AnyhowResult;
use chrono::{DateTime, Duration, Utc};
use rusqlite::{params, OptionalExtension};
use std::collections::HashMap;

use super::connection::DatabaseManager;

pub struct AnalyticsRepository {
    db: DatabaseManager,
}

impl AnalyticsRepository {
    pub fn new(db: DatabaseManager) -> Self {
        Self { db }
    }

    pub fn get_daily_usage_stats(&self, date: DateTime<Utc>) -> AnyhowResult<DailyUsageStats> {
        let start_of_day = date.date_naive().and_hms_opt(0, 0, 0).unwrap().and_utc();
        let end_of_day = start_of_day + Duration::days(1);

        self.db.with_connection(|conn| {
            let total_sessions: u32 = conn.query_row(
                "SELECT COUNT(*) FROM chat_sessions
                 WHERE created_at >= ?1 AND created_at < ?2",
                params![start_of_day.to_rfc3339(), end_of_day.to_rfc3339()],
                |row| row.get::<_, u32>(0),
            )?;

            let total_messages: u32 = conn.query_row(
                "SELECT COUNT(*) FROM messages m
                 JOIN chat_sessions cs ON m.session_id = cs.id
                 WHERE cs.created_at >= ?1 AND cs.created_at < ?2",
                params![start_of_day.to_rfc3339(), end_of_day.to_rfc3339()],
                |row| row.get::<_, u32>(0),
            )?;

            let total_tokens: u64 = conn.query_row(
                "SELECT COALESCE(SUM(token_count), 0) FROM chat_sessions
                 WHERE created_at >= ?1 AND created_at < ?2",
                params![start_of_day.to_rfc3339(), end_of_day.to_rfc3339()],
                |row| row.get::<_, u64>(0),
            )?;

            let mut stmt = conn.prepare(
                "SELECT provider, COUNT(*) as count
                 FROM chat_sessions
                 WHERE created_at >= ?1 AND created_at < ?2
                 GROUP BY provider",
            )?;

            let provider_iter = stmt.query_map(
                params![start_of_day.to_rfc3339(), end_of_day.to_rfc3339()],
                |row| {
                    let provider: String = row.get(0)?;
                    let count: u32 = row.get(1)?;
                    Ok((provider, count))
                },
            )?;

            let mut provider_usage = HashMap::new();
            for result in provider_iter {
                let (provider, count) = result?;
                provider_usage.insert(provider, count);
            }

            // Get hourly distribution
            let mut hourly_stmt = conn.prepare(
                "SELECT strftime('%H', created_at) as hour, COUNT(*) as count
                 FROM chat_sessions
                 WHERE created_at >= ?1 AND created_at < ?2
                 GROUP BY hour
                 ORDER BY hour",
            )?;

            let hourly_iter = hourly_stmt.query_map(
                params![start_of_day.to_rfc3339(), end_of_day.to_rfc3339()],
                |row| {
                    let hour: String = row.get(0)?;
                    let count: u32 = row.get(1)?;
                    Ok((hour.parse::<u8>().unwrap_or(0), count))
                },
            )?;

            let mut hourly_distribution = vec![0u32; 24];
            for result in hourly_iter {
                let (hour, count) = result?;
                if (hour as usize) < 24 {
                    hourly_distribution[hour as usize] = count;
                }
            }

            Ok(DailyUsageStats {
                date,
                total_sessions,
                total_messages,
                total_tokens,
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
        })
    }

    pub fn get_provider_usage_trends(&self, days: u32) -> AnyhowResult<Vec<ProviderTrend>> {
        let end_date = Utc::now();
        let start_date = end_date - Duration::days(days as i64);

        self.db.with_connection(|conn| {
            let mut stmt = conn.prepare(
                "SELECT provider,
                        DATE(created_at) as date,
                        COUNT(*) as session_count,
                        COALESCE(SUM(token_count), 0) as token_count
                 FROM chat_sessions
                 WHERE created_at >= ?1 AND created_at <= ?2
                 GROUP BY provider, DATE(created_at)
                 ORDER BY provider, date",
            )?;

            let trend_iter = stmt.query_map(
                params![start_date.to_rfc3339(), end_date.to_rfc3339()],
                |row| {
                    let provider: String = row.get(0)?;
                    let date_str: String = row.get(1)?;
                    let session_count: u32 = row.get(2)?;
                    let token_count: u64 = row.get(3)?;
                    Ok((provider, date_str, session_count, token_count))
                },
            )?;

            let mut provider_data: HashMap<String, Vec<DailyPoint>> = HashMap::new();
            for result in trend_iter {
                let (provider, date_str, session_count, token_count) = result?;
                let date = DateTime::parse_from_str(
                    &format!("{date_str}T00:00:00Z"),
                    "%Y-%m-%dT%H:%M:%SZ",
                )
                .unwrap()
                .with_timezone(&Utc);

                provider_data.entry(provider).or_default().push(DailyPoint {
                    date,
                    session_count,
                    token_count,
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
        })
    }

    pub fn get_session_length_distribution(&self) -> AnyhowResult<SessionLengthDistribution> {
        self.db.with_connection(|conn| {
            let mut stmt = conn.prepare(
                "SELECT
                    CASE
                        WHEN message_count <= 5 THEN 'short'
                        WHEN message_count <= 20 THEN 'medium'
                        WHEN message_count <= 50 THEN 'long'
                        ELSE 'very_long'
                    END as length_category,
                    COUNT(*) as count
                 FROM chat_sessions
                 GROUP BY length_category",
            )?;

            let dist_iter = stmt.query_map([], |row| {
                let category: String = row.get(0)?;
                let count: u32 = row.get(1)?;
                Ok((category, count))
            })?;

            let mut short = 0;
            let mut medium = 0;
            let mut long = 0;
            let mut very_long = 0;

            for result in dist_iter {
                let (category, count) = result?;
                match category.as_str() {
                    "short" => short = count,
                    "medium" => medium = count,
                    "long" => long = count,
                    "very_long" => very_long = count,
                    _ => {}
                }
            }

            let total = short + medium + long + very_long;

            Ok(SessionLengthDistribution {
                short_sessions: short,
                medium_sessions: medium,
                long_sessions: long,
                very_long_sessions: very_long,
                short_percentage: if total > 0 {
                    short as f64 / total as f64 * 100.0
                } else {
                    0.0
                },
                medium_percentage: if total > 0 {
                    medium as f64 / total as f64 * 100.0
                } else {
                    0.0
                },
                long_percentage: if total > 0 {
                    long as f64 / total as f64 * 100.0
                } else {
                    0.0
                },
                very_long_percentage: if total > 0 {
                    very_long as f64 / total as f64 * 100.0
                } else {
                    0.0
                },
            })
        })
    }

    pub fn get_most_active_hours(&self) -> AnyhowResult<Vec<HourlyActivity>> {
        self.db.with_connection(|conn| {
            let mut stmt = conn.prepare(
                "SELECT strftime('%H', created_at) as hour,
                        COUNT(*) as session_count,
                        COALESCE(SUM(token_count), 0) as token_count
                 FROM chat_sessions
                 GROUP BY hour
                 ORDER BY hour",
            )?;

            let activity_iter = stmt.query_map([], |row| {
                let hour_str: String = row.get(0)?;
                let session_count: u32 = row.get(1)?;
                let token_count: u64 = row.get(2)?;

                let hour = hour_str.parse::<u8>().unwrap_or(0);

                Ok(HourlyActivity {
                    hour,
                    session_count,
                    token_count,
                })
            })?;

            let mut activities = Vec::new();
            for activity in activity_iter {
                activities.push(activity?);
            }
            Ok(activities)
        })
    }

    pub fn generate_insights(&self, period_days: u32) -> AnyhowResult<Vec<String>> {
        let end_date = Utc::now();
        let start_date = end_date - Duration::days(period_days as i64);

        self.db.with_connection(|conn| {
            let mut insights = Vec::new();

            // Most used provider insight
            let top_provider: Option<(String, u32)> = conn
                .query_row(
                    "SELECT provider, COUNT(*) as count
                 FROM chat_sessions
                 WHERE created_at >= ?1 AND created_at <= ?2
                 GROUP BY provider
                 ORDER BY count DESC
                 LIMIT 1",
                    params![start_date.to_rfc3339(), end_date.to_rfc3339()],
                    |row| Ok((row.get::<_, String>(0)?, row.get::<_, u32>(1)?)),
                )
                .optional()?;

            if let Some((provider, count)) = top_provider {
                insights.push(format!(
                    "Your most used provider is {provider} with {count} sessions"
                ));
            }

            // Peak usage time insight
            let peak_hour: Option<u8> = conn
                .query_row(
                    "SELECT strftime('%H', created_at) as hour
                 FROM chat_sessions
                 WHERE created_at >= ?1 AND created_at <= ?2
                 GROUP BY hour
                 ORDER BY COUNT(*) DESC
                 LIMIT 1",
                    params![start_date.to_rfc3339(), end_date.to_rfc3339()],
                    |row| {
                        let hour_str: String = row.get(0)?;
                        Ok(hour_str.parse::<u8>().unwrap_or(0))
                    },
                )
                .optional()?;

            if let Some(hour) = peak_hour {
                insights.push(format!("Your peak usage time is {hour}:00"));
            }

            // Token usage insight
            let total_tokens: u64 = conn.query_row(
                "SELECT COALESCE(SUM(token_count), 0)
                 FROM chat_sessions
                 WHERE created_at >= ?1 AND created_at <= ?2",
                params![start_date.to_rfc3339(), end_date.to_rfc3339()],
                |row| row.get::<_, u64>(0),
            )?;

            if total_tokens > 0 {
                insights.push(format!(
                    "You've used {total_tokens} tokens in the last {period_days} days"
                ));
            }

            // Session length insight
            let avg_messages: f64 = conn
                .query_row(
                    "SELECT AVG(message_count)
                 FROM chat_sessions
                 WHERE created_at >= ?1 AND created_at <= ?2",
                    params![start_date.to_rfc3339(), end_date.to_rfc3339()],
                    |row| row.get::<_, f64>(0),
                )
                .unwrap_or(0.0);

            if avg_messages > 0.0 {
                let length_category = if avg_messages < 5.0 {
                    "short"
                } else if avg_messages < 20.0 {
                    "medium"
                } else {
                    "long"
                };
                insights.push(format!(
                    "Your average session has {avg_messages:.1} messages ({length_category})"
                ));
            }

            Ok(insights)
        })
    }
}

#[derive(Debug)]
pub struct DailyUsageStats {
    pub date: DateTime<Utc>,
    pub total_sessions: u32,
    pub total_messages: u32,
    pub total_tokens: u64,
    pub provider_usage: HashMap<String, u32>,
    pub hourly_distribution: Vec<u32>, // 24 hours
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::ChatSessionRepository;
    use crate::models::chat_session::{ChatSession, LlmProvider};

    #[test]
    fn test_daily_usage_stats() {
        let db = DatabaseManager::open_in_memory().unwrap();
        let repo = AnalyticsRepository::new(db.clone());

        // Create test session
        let session = ChatSession::new(
            LlmProvider::ClaudeCode,
            "test.jsonl".to_string(),
            "hash123".to_string(),
            Utc::now(),
        );

        let session_repo = ChatSessionRepository::new(db);
        session_repo.create(&session).unwrap();

        let today = Utc::now();
        let stats = repo.get_daily_usage_stats(today).unwrap();

        assert_eq!(stats.total_sessions, 1);
    }

    #[test]
    fn test_session_length_distribution() {
        let db = DatabaseManager::open_in_memory().unwrap();
        let repo = AnalyticsRepository::new(db.clone());

        let session_repo = ChatSessionRepository::new(db);

        // Create sessions with different lengths
        let mut short_session = ChatSession::new(
            LlmProvider::ClaudeCode,
            "short.jsonl".to_string(),
            "hash1".to_string(),
            Utc::now(),
        );
        short_session.message_count = 3;

        let mut medium_session = ChatSession::new(
            LlmProvider::Gemini,
            "medium.json".to_string(),
            "hash2".to_string(),
            Utc::now(),
        );
        medium_session.message_count = 15;

        let mut long_session = ChatSession::new(
            LlmProvider::ChatGpt,
            "long.json".to_string(),
            "hash3".to_string(),
            Utc::now(),
        );
        long_session.message_count = 35;

        session_repo.create(&short_session).unwrap();
        session_repo.create(&medium_session).unwrap();
        session_repo.create(&long_session).unwrap();

        let distribution = repo.get_session_length_distribution().unwrap();

        assert_eq!(distribution.short_sessions, 1);
        assert_eq!(distribution.medium_sessions, 1);
        assert_eq!(distribution.long_sessions, 1);
        assert_eq!(distribution.very_long_sessions, 0);
    }

    #[test]
    fn test_generate_insights() {
        let db = DatabaseManager::open_in_memory().unwrap();
        let repo = AnalyticsRepository::new(db.clone());

        let session_repo = ChatSessionRepository::new(db);

        // Create test sessions
        let session = ChatSession::new(
            LlmProvider::ClaudeCode,
            "test.jsonl".to_string(),
            "hash123".to_string(),
            Utc::now(),
        );

        session_repo.create(&session).unwrap();

        let insights = repo.generate_insights(7).unwrap();

        assert!(!insights.is_empty());
        assert!(insights
            .iter()
            .any(|insight| insight.contains("claude-code")));
    }
}
