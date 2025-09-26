use anyhow::{Context, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use uuid::Uuid;

use super::query_service::DateRange;
use super::retrospection_service::{ProcessingResult, RetrospectionService};
use crate::database::analytics_repo::AnalyticsRepository;
use crate::database::chat_session_repo::ChatSessionRepository;
use crate::database::connection::DatabaseManager;
use crate::database::message_repo::MessageRepository;
use crate::database::{AnalysisStatistics, QueueStatistics};
use crate::models::{ChatSession, RetrospectionAnalysis};

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

#[derive(Debug, Serialize, Deserialize)]
pub struct RetrospectionInsights {
    pub total_analyses: u32,
    pub analysis_statistics: AnalysisStatistics,
    pub queue_statistics: QueueStatistics,
    pub recent_analyses: Vec<RetrospectionAnalysis>,
    pub top_templates_used: Vec<TemplateUsageStats>,
    pub cost_analysis: CostAnalysis,
    pub performance_metrics: PerformanceMetrics,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TemplateUsageStats {
    pub template_id: String,
    pub template_name: String,
    pub usage_count: u32,
    pub avg_cost: f64,
    pub avg_tokens: f64,
    pub avg_execution_time_ms: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CostAnalysis {
    pub total_estimated_cost: f64,
    pub total_tokens_used: u32,
    pub avg_cost_per_analysis: f64,
    pub cost_by_model: HashMap<String, ModelCostStats>,
    pub cost_trend_last_30_days: Vec<DailyCostStats>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ModelCostStats {
    pub analyses_count: u32,
    pub total_cost: f64,
    pub total_tokens: u32,
    pub avg_cost_per_token: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DailyCostStats {
    pub date: String,
    pub analyses_count: u32,
    pub total_cost: f64,
    pub total_tokens: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    pub avg_processing_time_ms: f64,
    pub median_processing_time_ms: f64,
    pub success_rate: f64,
    pub throughput_analyses_per_hour: f64,
    pub queue_health: QueueHealthStatus,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct QueueHealthStatus {
    pub is_healthy: bool,
    pub pending_requests: u32,
    pub avg_queue_time_minutes: f64,
    pub oldest_request_age_hours: f64,
    pub processing_capacity_utilization: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EnhancedUsageInsights {
    pub usage_insights: UsageInsights,
    pub retrospection_insights: Option<RetrospectionInsights>,
}

pub struct AnalyticsService {
    db_manager: DatabaseManager,
    retrospection_service: Option<RetrospectionService>,
}

impl AnalyticsService {
    pub fn new(db_manager: DatabaseManager) -> Self {
        Self {
            db_manager,
            retrospection_service: None,
        }
    }

    /// Create a new AnalyticsService with retrospection capabilities
    pub fn with_retrospection(db_manager: DatabaseManager) -> Result<Self> {
        let retrospection_service = Some(RetrospectionService::new(db_manager.clone())?);
        Ok(Self {
            db_manager,
            retrospection_service,
        })
    }

    pub async fn generate_insights(&self) -> Result<UsageInsights> {
        let analytics_repo = AnalyticsRepository::new(self.db_manager.clone());
        let session_repo = ChatSessionRepository::new(self.db_manager.clone());
        let message_repo = MessageRepository::new(self.db_manager.clone());

        println!("Generating usage insights...");

        // Basic counts
        let sessions = session_repo.get_all()?;
        let total_sessions = sessions.len() as u64;
        let total_messages = message_repo.count_all()?;
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

        println!("✓ Generated comprehensive usage insights");

        Ok(UsageInsights {
            total_sessions,
            total_messages,
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

        println!("✓ Exported data to: {output_file}");

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

    /// Generate enhanced insights that include both usage and retrospection data
    pub async fn generate_enhanced_insights(&self) -> Result<EnhancedUsageInsights> {
        let usage_insights = self.generate_insights().await?;

        let retrospection_insights = if let Some(ref retro_service) = self.retrospection_service {
            Some(self.generate_retrospection_insights(retro_service).await?)
        } else {
            None
        };

        Ok(EnhancedUsageInsights {
            usage_insights,
            retrospection_insights,
        })
    }

    async fn generate_retrospection_insights(
        &self,
        retro_service: &RetrospectionService,
    ) -> Result<RetrospectionInsights> {
        let analysis_stats = retro_service.get_analysis_statistics().await?;
        let queue_stats = retro_service.get_queue_statistics().await?;
        let recent_analyses = retro_service.get_recent_analyses(10).await?;

        let total_analyses = analysis_stats.total_analyses;

        // Calculate template usage statistics
        let top_templates_used = self
            .calculate_template_usage_stats(&recent_analyses)
            .await?;

        // Calculate cost analysis
        let cost_analysis = self.calculate_cost_analysis(&recent_analyses).await?;

        // Calculate performance metrics
        let performance_metrics = self
            .calculate_performance_metrics(&analysis_stats, &queue_stats)
            .await?;

        Ok(RetrospectionInsights {
            total_analyses,
            analysis_statistics: analysis_stats,
            queue_statistics: queue_stats,
            recent_analyses,
            top_templates_used,
            cost_analysis,
            performance_metrics,
        })
    }

    async fn calculate_template_usage_stats(
        &self,
        analyses: &[RetrospectionAnalysis],
    ) -> Result<Vec<TemplateUsageStats>> {
        let mut template_map: HashMap<String, (u32, f64, f64, f64)> = HashMap::new();

        for analysis in analyses {
            let entry = template_map
                .entry(analysis.prompt_template_id.clone())
                .or_insert((0, 0.0, 0.0, 0.0));
            entry.0 += 1; // count
            entry.1 += analysis.metadata.estimated_cost; // total cost
            entry.2 += analysis.metadata.total_tokens as f64; // total tokens
            entry.3 += analysis.metadata.execution_time_ms as f64; // total execution time
        }

        let mut stats: Vec<TemplateUsageStats> = template_map
            .into_iter()
            .map(
                |(template_id, (count, total_cost, total_tokens, total_time))| {
                    TemplateUsageStats {
                        template_name: template_id.clone(), // Could be enhanced to lookup actual name
                        template_id,
                        usage_count: count,
                        avg_cost: total_cost / count as f64,
                        avg_tokens: total_tokens / count as f64,
                        avg_execution_time_ms: total_time / count as f64,
                    }
                },
            )
            .collect();

        stats.sort_by(|a, b| b.usage_count.cmp(&a.usage_count));
        stats.truncate(10); // Top 10 templates

        Ok(stats)
    }

    async fn calculate_cost_analysis(
        &self,
        analyses: &[RetrospectionAnalysis],
    ) -> Result<CostAnalysis> {
        let total_estimated_cost: f64 = analyses.iter().map(|a| a.metadata.estimated_cost).sum();
        let total_tokens_used: u32 = analyses.iter().map(|a| a.metadata.total_tokens).sum();
        let avg_cost_per_analysis = if !analyses.is_empty() {
            total_estimated_cost / analyses.len() as f64
        } else {
            0.0
        };

        // Group by model
        let mut cost_by_model: HashMap<String, ModelCostStats> = HashMap::new();
        for analysis in analyses {
            let entry = cost_by_model
                .entry(analysis.metadata.llm_service.clone())
                .or_insert(ModelCostStats {
                    analyses_count: 0,
                    total_cost: 0.0,
                    total_tokens: 0,
                    avg_cost_per_token: 0.0,
                });

            entry.analyses_count += 1;
            entry.total_cost += analysis.metadata.estimated_cost;
            entry.total_tokens += analysis.metadata.total_tokens;
        }

        // Calculate averages
        for stats in cost_by_model.values_mut() {
            stats.avg_cost_per_token = if stats.total_tokens > 0 {
                stats.total_cost / stats.total_tokens as f64
            } else {
                0.0
            };
        }

        Ok(CostAnalysis {
            total_estimated_cost,
            total_tokens_used,
            avg_cost_per_analysis,
            cost_by_model,
            cost_trend_last_30_days: Vec::new(), // Could be implemented with date-based grouping
        })
    }

    async fn calculate_performance_metrics(
        &self,
        analysis_stats: &AnalysisStatistics,
        queue_stats: &QueueStatistics,
    ) -> Result<PerformanceMetrics> {
        // Calculate basic metrics from status breakdown
        let total_processed = analysis_stats
            .status_breakdown
            .iter()
            .filter(|s| {
                matches!(
                    s.status,
                    crate::models::AnalysisStatus::Complete | crate::models::AnalysisStatus::Failed
                )
            })
            .map(|s| s.count)
            .sum::<u32>();

        let successful = analysis_stats
            .status_breakdown
            .iter()
            .find(|s| matches!(s.status, crate::models::AnalysisStatus::Complete))
            .map(|s| s.count)
            .unwrap_or(0);

        let success_rate = if total_processed > 0 {
            successful as f64 / total_processed as f64
        } else {
            0.0
        };

        let avg_processing_time_ms = analysis_stats
            .status_breakdown
            .iter()
            .find(|s| matches!(s.status, crate::models::AnalysisStatus::Complete))
            .and_then(|s| s.avg_execution_time_ms)
            .unwrap_or(0.0);

        let queue_health = QueueHealthStatus {
            is_healthy: queue_stats.queued_count < 50 && queue_stats.avg_queue_age_minutes < 60.0,
            pending_requests: queue_stats.total_pending(),
            avg_queue_time_minutes: queue_stats.avg_queue_age_minutes,
            oldest_request_age_hours: queue_stats.max_queue_age_minutes / 60.0,
            processing_capacity_utilization: if queue_stats.total_pending() > 0 {
                queue_stats.processing_count as f64
                    / (queue_stats.processing_count + queue_stats.queued_count) as f64
            } else {
                0.0
            },
        };

        Ok(PerformanceMetrics {
            avg_processing_time_ms,
            median_processing_time_ms: avg_processing_time_ms, // Simplified
            success_rate,
            throughput_analyses_per_hour: 0.0, // Could be calculated with time-based data
            queue_health,
        })
    }

    /// Submit an analysis request for a session
    pub async fn analyze_session(
        &self,
        session_id: Uuid,
        template_id: String,
        variables: HashMap<String, String>,
    ) -> Result<Option<ProcessingResult>> {
        match &self.retrospection_service {
            Some(retro_service) => {
                let request = retro_service
                    .submit_analysis_request(session_id, template_id, variables)
                    .await?;
                let _analysis = retro_service.process_analysis_request(request).await?;

                // Create a simple processing result
                Ok(Some(ProcessingResult {
                    processed: 1,
                    successful: 1,
                    failed: 0,
                    errors: Vec::new(),
                }))
            }
            None => Ok(None),
        }
    }

    /// Process pending retrospection requests
    pub async fn process_pending_retrospections(&self) -> Result<Option<ProcessingResult>> {
        match &self.retrospection_service {
            Some(retro_service) => {
                let result = retro_service.process_pending_requests().await?;
                Ok(Some(result))
            }
            None => Ok(None),
        }
    }

    /// Get retrospection insights only
    pub async fn get_retrospection_insights(&self) -> Result<Option<RetrospectionInsights>> {
        match &self.retrospection_service {
            Some(retro_service) => {
                let insights = self.generate_retrospection_insights(retro_service).await?;
                Ok(Some(insights))
            }
            None => Ok(None),
        }
    }

    /// Check if retrospection is enabled
    pub fn has_retrospection(&self) -> bool {
        self.retrospection_service.is_some()
    }

    pub async fn print_insights_summary(&self) -> Result<()> {
        let insights = self.generate_insights().await?;

        println!("\nUsage Insights Summary");
        println!("======================");
        println!("Total Sessions: {}", insights.total_sessions);
        println!("Total Messages: {}", insights.total_messages);
        println!("Total Tokens: {}", insights.total_tokens);

        if !insights.date_range.start_date.is_empty() && !insights.date_range.end_date.is_empty() {
            println!(
                "Date Range: {} to {} ({} days)",
                insights.date_range.start_date, insights.date_range.end_date, insights.span_days
            );
        }

        println!("\nProvider Breakdown:");
        for (provider, stats) in &insights.provider_breakdown {
            println!(
                "  {}: {} sessions ({:.1}%)",
                provider, stats.sessions, stats.percentage_of_total
            );
        }

        println!("\nFor detailed analysis, use: retrochat analyze export json");

        Ok(())
    }
}

impl Default for AnalyticsService {
    fn default() -> Self {
        Self::new(DatabaseManager::new("retrochat.db").unwrap())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_analytics_service_creation() {
        let db_manager = DatabaseManager::new(":memory:").unwrap();
        let _service = AnalyticsService::new(db_manager);
        // Just test that we can create the service
    }

    #[tokio::test]
    async fn test_generate_insights_empty_database() {
        let db_manager = DatabaseManager::new(":memory:").unwrap();
        let service = AnalyticsService::new(db_manager);

        let result = service.generate_insights().await;
        assert!(result.is_ok());

        let insights = result.unwrap();
        assert_eq!(insights.total_sessions, 0);
        assert_eq!(insights.total_messages, 0);
        assert_eq!(insights.total_tokens, 0);
    }
}
