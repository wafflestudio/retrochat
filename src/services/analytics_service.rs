use anyhow::{Context, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;

use super::query_service::DateRange;
use crate::database::{
    AnalyticsRepository, ChatSessionRepository, DatabaseManager, MessageRepository,
};
use crate::models::ChatSession;

#[derive(Debug, Serialize, Deserialize)]
pub struct UsageInsights {
    pub total_sessions: u64,
    pub total_messages: u64,
    pub total_tokens: u64,
    pub date_range: DateRange,
    pub span_days: i64,
    pub provider_breakdown: HashMap<String, ProviderStats>,
    pub daily_activity: Vec<DailyActivity>,
    pub message_role_distribution: MessageRoleDistribution,
    pub top_projects: Vec<ProjectStats>,
    pub session_duration_stats: DurationStats,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProviderStats {
    pub sessions: u64,
    pub messages: u64,
    pub tokens: u64,
    pub percentage_of_total: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DailyActivity {
    pub date: String,
    pub sessions: u64,
    pub messages: u64,
    pub tokens: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MessageRoleDistribution {
    pub user_messages: u64,
    pub assistant_messages: u64,
    pub system_messages: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProjectStats {
    pub name: String,
    pub sessions: u64,
    pub messages: u64,
    pub tokens: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DurationStats {
    pub avg_duration_minutes: f64,
    pub median_duration_minutes: f64,
    pub max_duration_minutes: f64,
    pub sessions_with_duration: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportRequest {
    pub format: String,
    pub data_types: Vec<String>,
    pub date_range: Option<DateRange>,
    pub filters: Option<ExportFilters>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportFilters {
    pub providers: Option<Vec<String>>,
    pub projects: Option<Vec<String>>,
    pub include_content: Option<bool>,
    pub min_message_length: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExportResponse {
    pub export_id: String,
    pub format: String,
    pub file_path: String,
    pub file_size_bytes: i64,
    pub export_duration_ms: i32,
    pub records_exported: i32,
    pub compression_used: bool,
}

pub struct AnalyticsService {
    db_manager: DatabaseManager,
}

impl AnalyticsService {
    pub fn new(db_manager: DatabaseManager) -> Self {
        Self { db_manager }
    }

    pub async fn generate_insights(&self) -> Result<UsageInsights> {
        let analytics_repo = AnalyticsRepository::new(&self.db_manager);
        let session_repo = ChatSessionRepository::new(&self.db_manager);
        let message_repo = MessageRepository::new(&self.db_manager);

        tracing::info!("Generating usage insights...");

        // Basic counts
        let sessions = session_repo.get_all().await?;
        let total_sessions = sessions.len() as u64;
        let total_messages = message_repo.count_all().await?;
        let total_tokens = sessions
            .iter()
            .map(|s| s.token_count.unwrap_or(0) as u64)
            .sum();

        if total_sessions == 0 {
            return Ok(UsageInsights {
                total_sessions: 0,
                total_messages: 0,
                total_tokens: 0,
                date_range: DateRange {
                    start_date: "".to_string(),
                    end_date: "".to_string(),
                },
                span_days: 0,
                provider_breakdown: HashMap::new(),
                daily_activity: Vec::new(),
                message_role_distribution: MessageRoleDistribution {
                    user_messages: 0,
                    assistant_messages: 0,
                    system_messages: 0,
                },
                top_projects: Vec::new(),
                session_duration_stats: DurationStats {
                    avg_duration_minutes: 0.0,
                    median_duration_minutes: 0.0,
                    max_duration_minutes: 0.0,
                    sessions_with_duration: 0,
                },
            });
        }

        // Date range
        let (date_range, span_days) = self.calculate_date_range(&sessions).await?;

        // Provider breakdown
        let provider_breakdown = self
            .calculate_provider_breakdown(&sessions, total_sessions)
            .await?;

        // Daily activity
        let daily_activity = self.calculate_daily_activity(&analytics_repo).await?;

        // Message role distribution
        let message_role_distribution = self
            .calculate_message_role_distribution(&analytics_repo)
            .await?;

        // Top projects
        let top_projects = self.calculate_top_projects(&sessions).await?;

        // Session duration stats
        let session_duration_stats = self.calculate_duration_stats(&sessions).await?;

        tracing::info!("Generated comprehensive usage insights");

        Ok(UsageInsights {
            total_sessions,
            total_messages: total_messages as u64,
            total_tokens,
            date_range,
            span_days,
            provider_breakdown,
            daily_activity,
            message_role_distribution,
            top_projects,
            session_duration_stats,
        })
    }

    async fn calculate_date_range(&self, sessions: &[ChatSession]) -> Result<(DateRange, i64)> {
        let earliest = sessions.iter().map(|s| s.created_at).min();
        let latest = sessions
            .iter()
            .map(|s| s.end_time.unwrap_or(s.created_at))
            .max();

        let span_days = if let (Some(early), Some(late)) = (&earliest, &latest) {
            (late.date_naive() - early.date_naive()).num_days()
        } else {
            0
        };

        let date_range = if let (Some(early), Some(late)) = (&earliest, &latest) {
            DateRange {
                start_date: early.format("%Y-%m-%d").to_string(),
                end_date: late.format("%Y-%m-%d").to_string(),
            }
        } else {
            DateRange {
                start_date: "".to_string(),
                end_date: "".to_string(),
            }
        };

        Ok((date_range, span_days))
    }

    async fn calculate_provider_breakdown(
        &self,
        sessions: &[ChatSession],
        total_sessions: u64,
    ) -> Result<HashMap<String, ProviderStats>> {
        // Calculate provider stats from sessions
        let mut provider_map = std::collections::HashMap::new();
        for session in sessions {
            let entry = provider_map
                .entry(session.provider.clone())
                .or_insert((0u32, 0u32, 0u32));
            entry.0 += 1; // sessions
            entry.1 += session.message_count; // messages
            entry.2 += session.token_count.unwrap_or(0); // tokens
        }
        let provider_stats: Vec<_> = provider_map
            .into_iter()
            .map(|(provider, (sessions, messages, tokens))| (provider, sessions, messages, tokens))
            .collect();
        let mut breakdown = HashMap::new();

        for (provider, sessions, messages, tokens) in provider_stats {
            let percentage = if total_sessions > 0 {
                (sessions as f64 / total_sessions as f64) * 100.0
            } else {
                0.0
            };

            breakdown.insert(
                provider.to_string(),
                ProviderStats {
                    sessions: sessions as u64,
                    messages: messages as u64,
                    tokens: tokens as u64,
                    percentage_of_total: percentage,
                },
            );
        }

        Ok(breakdown)
    }

    async fn calculate_daily_activity(
        &self,
        _analytics_repo: &AnalyticsRepository,
    ) -> Result<Vec<DailyActivity>> {
        // For now, return empty daily activity (would need proper date grouping)
        let daily_stats: Vec<(String, u32, u32, u32)> = Vec::new();

        Ok(daily_stats
            .into_iter()
            .map(|(date, sessions, messages, tokens)| DailyActivity {
                date,
                sessions: sessions as u64,
                messages: messages as u64,
                tokens: tokens as u64,
            })
            .collect())
    }

    async fn calculate_message_role_distribution(
        &self,
        _analytics_repo: &AnalyticsRepository,
    ) -> Result<MessageRoleDistribution> {
        // For now, use a simple estimate since we don't have accurate message count
        let total_messages = 100u64; // Placeholder estimate

        // For now, use placeholder role distribution (would need message repo enhancement)
        let role_stats = vec![
            ("user".to_string(), total_messages / 2), // Rough estimate
            ("assistant".to_string(), total_messages / 2),
            ("system".to_string(), 0),
        ];

        let mut user_messages = 0;
        let mut assistant_messages = 0;
        let mut system_messages = 0;

        for (role, count) in role_stats {
            match role.as_str() {
                "user" => user_messages = count,
                "assistant" => assistant_messages = count,
                "system" => system_messages = count,
                _ => {}
            }
        }

        Ok(MessageRoleDistribution {
            user_messages,
            assistant_messages,
            system_messages,
        })
    }

    async fn calculate_top_projects(&self, sessions: &[ChatSession]) -> Result<Vec<ProjectStats>> {
        // Get project stats from sessions
        let mut project_map = std::collections::HashMap::new();
        for session in sessions {
            let project_name = session.project_name.clone();
            let entry = project_map
                .entry(project_name)
                .or_insert((0u32, 0u32, 0u32));
            entry.0 += 1; // sessions
            entry.1 += session.message_count; // messages
            entry.2 += session.token_count.unwrap_or(0); // tokens
        }
        let project_stats: Vec<_> = project_map
            .into_iter()
            .map(|(name, (sessions, messages, tokens))| (name, sessions, messages, tokens))
            .collect();

        Ok(project_stats
            .into_iter()
            .map(|(name, sessions, messages, tokens)| ProjectStats {
                name: name.unwrap_or("Unnamed".to_string()),
                sessions: sessions as u64,
                messages: messages as u64,
                tokens: tokens as u64,
            })
            .take(10) // Top 10 projects
            .collect())
    }

    async fn calculate_duration_stats(&self, sessions: &[ChatSession]) -> Result<DurationStats> {
        // Calculate session durations
        let durations: Vec<f64> = sessions
            .iter()
            .filter_map(|s| {
                s.end_time
                    .map(|end| (end - s.created_at).num_minutes() as f64)
            })
            .collect();

        if durations.is_empty() {
            return Ok(DurationStats {
                avg_duration_minutes: 0.0,
                median_duration_minutes: 0.0,
                max_duration_minutes: 0.0,
                sessions_with_duration: 0,
            });
        }

        let mut sorted_durations = durations.clone();
        sorted_durations.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let avg_duration_minutes = durations.iter().sum::<f64>() / durations.len() as f64;
        let median_duration_minutes = if sorted_durations.len().is_multiple_of(2) {
            let mid = sorted_durations.len() / 2;
            (sorted_durations[mid - 1] + sorted_durations[mid]) / 2.0
        } else {
            sorted_durations[sorted_durations.len() / 2]
        };
        let max_duration_minutes = sorted_durations.last().copied().unwrap_or(0.0);

        Ok(DurationStats {
            avg_duration_minutes,
            median_duration_minutes,
            max_duration_minutes,
            sessions_with_duration: durations.len() as u64,
        })
    }

    pub async fn export_data(
        &self,
        format: &str,
        output_path: Option<String>,
    ) -> Result<ExportResponse> {
        let insights = self.generate_insights().await?;

        let output_file = output_path.unwrap_or_else(|| {
            format!(
                "retrochat_export_{}.{}",
                Utc::now().format("%Y%m%d_%H%M%S"),
                format
            )
        });

        let start_time = std::time::Instant::now();

        let records_exported = match format.to_lowercase().as_str() {
            "json" => {
                self.export_json(&insights, &output_file).await?;
                insights.daily_activity.len() as i32
            }
            "csv" => {
                self.export_csv(&insights, &output_file).await?;
                insights.daily_activity.len() as i32
            }
            "txt" | "text" => {
                self.export_text(&insights, &output_file).await?;
                insights.daily_activity.len() as i32
            }
            _ => {
                return Err(anyhow::anyhow!(
                    "Unsupported export format: {format}. Supported formats: json, csv, txt"
                ))
            }
        };

        let file_metadata = fs::metadata(&output_file)?;
        let file_size = file_metadata.len() as i64;
        let export_duration = start_time.elapsed().as_millis() as i32;

        tracing::info!(output_file = %output_file, "Exported data successfully");

        Ok(ExportResponse {
            export_id: format!("export_{}", Utc::now().timestamp()),
            format: format.to_string(),
            file_path: output_file,
            file_size_bytes: file_size,
            export_duration_ms: export_duration,
            records_exported,
            compression_used: false,
        })
    }

    async fn export_json(&self, insights: &UsageInsights, output_file: &str) -> Result<()> {
        let json_content = serde_json::to_string_pretty(insights)
            .with_context(|| "Failed to serialize insights to JSON")?;

        fs::write(output_file, json_content)
            .with_context(|| format!("Failed to write JSON file: {output_file}"))?;

        Ok(())
    }

    async fn export_csv(&self, insights: &UsageInsights, output_file: &str) -> Result<()> {
        let mut csv_content = String::new();

        // Summary section
        csv_content.push_str("Section,Metric,Value\n");
        csv_content.push_str(&format!(
            "Summary,Total Sessions,{}\n",
            insights.total_sessions
        ));
        csv_content.push_str(&format!(
            "Summary,Total Messages,{}\n",
            insights.total_messages
        ));
        csv_content.push_str(&format!("Summary,Total Tokens,{}\n", insights.total_tokens));
        csv_content.push_str(&format!("Summary,Date Span Days,{}\n", insights.span_days));

        // Provider breakdown
        for (provider, stats) in &insights.provider_breakdown {
            csv_content.push_str(&format!(
                "Provider,{} Sessions,{}\n",
                provider, stats.sessions
            ));
            csv_content.push_str(&format!(
                "Provider,{} Messages,{}\n",
                provider, stats.messages
            ));
            csv_content.push_str(&format!("Provider,{} Tokens,{}\n", provider, stats.tokens));
        }

        fs::write(output_file, csv_content)
            .with_context(|| format!("Failed to write CSV file: {output_file}"))?;

        Ok(())
    }

    async fn export_text(&self, insights: &UsageInsights, output_file: &str) -> Result<()> {
        let mut content = String::new();

        content.push_str("RetroChat Usage Insights Report\n");
        content.push_str("===============================\n\n");

        content.push_str(&format!(
            "Generated: {}\n\n",
            Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
        ));

        content.push_str("SUMMARY\n");
        content.push_str("-------\n");
        content.push_str(&format!("Total Sessions: {}\n", insights.total_sessions));
        content.push_str(&format!("Total Messages: {}\n", insights.total_messages));
        content.push_str(&format!("Total Tokens: {}\n", insights.total_tokens));

        if !insights.date_range.start_date.is_empty() && !insights.date_range.end_date.is_empty() {
            content.push_str(&format!(
                "Date Range: {} to {} ({} days)\n",
                insights.date_range.start_date, insights.date_range.end_date, insights.span_days
            ));
        }

        content.push_str("\nPROVIDER BREAKDOWN\n");
        content.push_str("------------------\n");
        for (provider, stats) in &insights.provider_breakdown {
            content.push_str(&format!(
                "{}: {} sessions ({:.1}%), {} messages, {} tokens\n",
                provider, stats.sessions, stats.percentage_of_total, stats.messages, stats.tokens
            ));
        }

        content.push_str("\nMESSAGE ROLES\n");
        content.push_str("-------------\n");
        content.push_str(&format!(
            "User: {}\n",
            insights.message_role_distribution.user_messages
        ));
        content.push_str(&format!(
            "Assistant: {}\n",
            insights.message_role_distribution.assistant_messages
        ));
        content.push_str(&format!(
            "System: {}\n",
            insights.message_role_distribution.system_messages
        ));

        if !insights.top_projects.is_empty() {
            content.push_str("\nTOP PROJECTS\n");
            content.push_str("------------\n");
            for project in &insights.top_projects {
                content.push_str(&format!(
                    "{}: {} sessions, {} messages, {} tokens\n",
                    project.name, project.sessions, project.messages, project.tokens
                ));
            }
        }

        content.push_str("\nSESSION DURATION STATS\n");
        content.push_str("----------------------\n");
        content.push_str(&format!(
            "Average: {:.1} minutes\n",
            insights.session_duration_stats.avg_duration_minutes
        ));
        content.push_str(&format!(
            "Median: {:.1} minutes\n",
            insights.session_duration_stats.median_duration_minutes
        ));
        content.push_str(&format!(
            "Maximum: {:.1} minutes\n",
            insights.session_duration_stats.max_duration_minutes
        ));
        content.push_str(&format!(
            "Sessions with duration: {}\n",
            insights.session_duration_stats.sessions_with_duration
        ));

        fs::write(output_file, content)
            .with_context(|| format!("Failed to write text file: {output_file}"))?;

        Ok(())
    }
}

impl Default for AnalyticsService {
    fn default() -> Self {
        // Use a blocking approach for Default implementation
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                Self::new(DatabaseManager::new("./retrochat.db").await.unwrap())
            })
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_analytics_service_creation() {
        let db_manager = DatabaseManager::new(":memory:").await.unwrap();
        let _service = AnalyticsService::new(db_manager);
        // Just test that we can create the service
    }

    #[tokio::test]
    async fn test_generate_insights_empty_database() {
        let db_manager = DatabaseManager::new(":memory:").await.unwrap();
        let service = AnalyticsService::new(db_manager);

        let result = service.generate_insights().await;
        assert!(result.is_ok());

        let insights = result.unwrap();
        assert_eq!(insights.total_sessions, 0);
        assert_eq!(insights.total_messages, 0);
        assert_eq!(insights.total_tokens, 0);
    }
}
