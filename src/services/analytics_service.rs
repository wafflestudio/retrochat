use super::google_ai::GoogleAiClient;
use crate::database::{
    AnalyticsRepository, ChatSessionRepository, DatabaseManager, MessageRepository,
    ToolOperationRepository,
};
use crate::models::ChatSession;
use crate::services::query_service::DateRange;
use anyhow::Result;
use chrono::Utc;
use std::collections::HashMap;

// Import from analytics module
use super::analytics::{
    calculate_processed_code_metrics, calculate_processed_token_metrics, calculate_session_metrics,
    calculate_time_efficiency_metrics, collect_qualitative_data, collect_quantitative_data,
    generate_qualitative_analysis_ai, generate_qualitative_analysis_fallback,
    generate_quantitative_analysis_ai, generate_quantitative_analysis_fallback,
    ComprehensiveAnalysis, DurationStats, MessageRoleDistribution, ProcessedQuantitativeOutput,
    QuantitativeInput, UsageInsights,
};

pub struct AnalyticsService {
    db_manager: DatabaseManager,
    google_ai_client: Option<GoogleAiClient>,
}

impl AnalyticsService {
    pub fn new(db_manager: DatabaseManager) -> Self {
        Self {
            db_manager,
            google_ai_client: None,
        }
    }

    pub fn with_google_ai(mut self, google_ai_client: GoogleAiClient) -> Self {
        self.google_ai_client = Some(google_ai_client);
        self
    }

    // =============================================================================
    // Basic Analytics (기존 기능 유지)
    // =============================================================================

    pub async fn generate_usage_insights(&self) -> Result<UsageInsights> {
        tracing::info!("Generating usage insights...");

        let analytics_repo = AnalyticsRepository::new(&self.db_manager);

        // Get basic stats using existing methods
        let (sessions, messages, tokens) = analytics_repo.get_total_stats().await?;
        let total_sessions = sessions as u64;
        let total_messages = messages as u64;
        let total_tokens = tokens as u64;

        // Create a simple date range (last 30 days)
        let end_date = chrono::Utc::now();
        let start_date = end_date - chrono::Duration::days(30);
        let date_range = DateRange {
            start_date: start_date.to_rfc3339(),
            end_date: end_date.to_rfc3339(),
        };
        let span_days = 30;

        // Create empty provider breakdown for now
        let provider_breakdown = HashMap::new();

        // Create empty daily activity for now
        let daily_activity = Vec::new();

        // Create empty message role distribution for now
        let message_role_distribution = MessageRoleDistribution {
            user_messages: 0,
            assistant_messages: 0,
            system_messages: 0,
        };

        // Create empty top projects for now
        let top_projects = Vec::new();

        // Create empty session duration stats for now
        let session_duration_stats = DurationStats {
            average_minutes: 0.0,
            median_minutes: 0.0,
            min_minutes: 0.0,
            max_minutes: 0.0,
        };

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

    pub async fn export_data(&self, format: &str, output_path: &str) -> Result<String> {
        let insights = self.generate_usage_insights().await?;

        match format.to_lowercase().as_str() {
            "json" => {
                let json = serde_json::to_string_pretty(&insights)?;
                std::fs::write(output_path, &json)?;
                Ok(format!("Data exported to {} in JSON format", output_path))
            }
            "csv" => {
                let csv = self.convert_to_csv(&insights)?;
                std::fs::write(output_path, &csv)?;
                Ok(format!("Data exported to {} in CSV format", output_path))
            }
            "txt" => {
                let text = self.convert_to_text(&insights)?;
                std::fs::write(output_path, &text)?;
                Ok(format!("Data exported to {} in text format", output_path))
            }
            _ => Err(anyhow::anyhow!("Unsupported format: {}", format)),
        }
    }

    // =============================================================================
    // Advanced Analytics (새로운 기능)
    // =============================================================================

    pub async fn analyze_session_comprehensive(
        &self,
        session_id: &str,
    ) -> Result<ComprehensiveAnalysis> {
        tracing::info!(
            "Starting comprehensive analysis for session: {}",
            session_id
        );

        // Get repositories
        let session_repo = ChatSessionRepository::new(&self.db_manager);
        let message_repo = MessageRepository::new(&self.db_manager);
        let tool_op_repo = ToolOperationRepository::new(&self.db_manager);

        // Parse session_id to UUID
        let session_uuid = uuid::Uuid::parse_str(session_id)
            .map_err(|e| anyhow::anyhow!("Invalid session ID format: {}", e))?;

        // Get session data
        let session = session_repo
            .get_by_id(&session_uuid)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Session not found: {}", session_id))?;

        // Get messages and tool operations
        let messages = message_repo.get_by_session(&session_uuid).await?;
        let tool_operations = tool_op_repo.get_by_session(&session_uuid).await?;

        // Collect quantitative and qualitative data
        let quantitative_input =
            collect_quantitative_data(&session, &messages, &tool_operations).await?;
        let qualitative_input =
            collect_qualitative_data(&tool_operations, &messages, &session).await?;

        // Generate analysis
        let quantitative_output = if let Some(ref ai_client) = self.google_ai_client {
            generate_quantitative_analysis_ai(&quantitative_input, ai_client).await?
        } else {
            generate_quantitative_analysis_fallback(&quantitative_input)?
        };

        let qualitative_output = if let Some(ref ai_client) = self.google_ai_client {
            generate_qualitative_analysis_ai(&qualitative_input, ai_client).await?
        } else {
            generate_qualitative_analysis_fallback(&qualitative_input)?
        };

        // Process quantitative data
        let processed_output = self
            .process_quantitative_data(&quantitative_input, &session)
            .await?;

        Ok(ComprehensiveAnalysis {
            session_id: session_id.to_string(),
            generated_at: Utc::now(),
            quantitative_input,
            qualitative_input,
            quantitative_output,
            qualitative_output,
            processed_output,
        })
    }

    async fn process_quantitative_data(
        &self,
        quantitative_input: &QuantitativeInput,
        _session: &ChatSession,
    ) -> Result<ProcessedQuantitativeOutput> {
        let session_duration_hours =
            quantitative_input.time_metrics.total_session_time_minutes / 60.0;

        // Calculate processed metrics
        let token_metrics = calculate_processed_token_metrics(
            quantitative_input.token_metrics.total_tokens_used,
            session_duration_hours,
            quantitative_input.token_metrics.input_tokens,
            quantitative_input.token_metrics.output_tokens,
        );

        let code_change_metrics = calculate_processed_code_metrics(
            quantitative_input.file_changes.net_code_growth,
            quantitative_input.file_changes.total_files_modified,
            session_duration_hours,
            quantitative_input.file_changes.refactoring_operations,
            quantitative_input.tool_usage.total_operations,
        );

        let time_efficiency_metrics = calculate_time_efficiency_metrics(
            session_duration_hours,
            session_duration_hours * 0.8, // Assume 80% productive time
            0,                            // TODO: Calculate context switches
        );

        let session_metrics = calculate_session_metrics(
            1,
            quantitative_input.time_metrics.total_session_time_minutes,
        );

        Ok(ProcessedQuantitativeOutput {
            session_metrics,
            token_metrics,
            code_change_metrics,
            time_efficiency_metrics,
        })
    }

    // =============================================================================
    // Helper Functions (기존 기능 유지)
    // =============================================================================

    fn convert_to_csv(&self, insights: &UsageInsights) -> Result<String> {
        let mut csv = String::new();
        csv.push_str("Metric,Value\n");
        csv.push_str(&format!("Total Sessions,{}\n", insights.total_sessions));
        csv.push_str(&format!("Total Messages,{}\n", insights.total_messages));
        csv.push_str(&format!("Total Tokens,{}\n", insights.total_tokens));
        csv.push_str(&format!("Span Days,{}\n", insights.span_days));
        csv.push_str(&format!(
            "Average Session Duration,{:.2}\n",
            insights.session_duration_stats.average_minutes
        ));
        csv.push_str(&format!(
            "Median Session Duration,{:.2}\n",
            insights.session_duration_stats.median_minutes
        ));
        csv.push_str(&format!(
            "Min Session Duration,{:.2}\n",
            insights.session_duration_stats.min_minutes
        ));
        csv.push_str(&format!(
            "Max Session Duration,{:.2}\n",
            insights.session_duration_stats.max_minutes
        ));
        csv.push_str(&format!(
            "User Messages,{}\n",
            insights.message_role_distribution.user_messages
        ));
        csv.push_str(&format!(
            "Assistant Messages,{}\n",
            insights.message_role_distribution.assistant_messages
        ));
        csv.push_str(&format!(
            "System Messages,{}\n",
            insights.message_role_distribution.system_messages
        ));
        Ok(csv)
    }

    fn convert_to_text(&self, insights: &UsageInsights) -> Result<String> {
        let mut text = String::new();
        text.push_str("=== Usage Insights ===\n\n");
        text.push_str(&format!("Total Sessions: {}\n", insights.total_sessions));
        text.push_str(&format!("Total Messages: {}\n", insights.total_messages));
        text.push_str(&format!("Total Tokens: {}\n", insights.total_tokens));
        text.push_str(&format!(
            "Date Range: {} to {}\n",
            insights.date_range.start_date, insights.date_range.end_date
        ));
        text.push_str(&format!("Span: {} days\n", insights.span_days));
        text.push_str(&format!(
            "Average Session Duration: {:.2} minutes\n",
            insights.session_duration_stats.average_minutes
        ));
        text.push_str(&format!(
            "Median Session Duration: {:.2} minutes\n",
            insights.session_duration_stats.median_minutes
        ));
        text.push_str(&format!(
            "Min Session Duration: {:.2} minutes\n",
            insights.session_duration_stats.min_minutes
        ));
        text.push_str(&format!(
            "Max Session Duration: {:.2} minutes\n",
            insights.session_duration_stats.max_minutes
        ));

        text.push_str("\n=== Message Role Distribution ===\n");
        text.push_str(&format!(
            "User Messages: {}\n",
            insights.message_role_distribution.user_messages
        ));
        text.push_str(&format!(
            "Assistant Messages: {}\n",
            insights.message_role_distribution.assistant_messages
        ));
        text.push_str(&format!(
            "System Messages: {}\n",
            insights.message_role_distribution.system_messages
        ));

        text.push_str("\n=== Provider Breakdown ===\n");
        for (provider, stats) in &insights.provider_breakdown {
            text.push_str(&format!(
                "{}: {} sessions, {} messages, {} tokens ({:.1}%)\n",
                provider, stats.sessions, stats.messages, stats.tokens, stats.percentage_of_total
            ));
        }

        text.push_str("\n=== Top Projects ===\n");
        for project in &insights.top_projects {
            text.push_str(&format!(
                "{}: {} sessions, {} messages, {} tokens\n",
                project.project_name, project.sessions, project.messages, project.tokens
            ));
        }

        Ok(text)
    }
}
