use super::google_ai::GoogleAiClient;
use crate::database::{
    ChatSessionRepository, DatabaseManager, MessageRepository,
    ToolOperationRepository,
};
use crate::models::ChatSession;
use anyhow::Result;
use chrono::Utc;
use std::sync::Arc;

// Import from analytics module
use super::analytics::{
    calculate_processed_code_metrics, calculate_processed_token_metrics, calculate_session_metrics,
    calculate_time_efficiency_metrics, collect_qualitative_data, collect_quantitative_data,
    generate_qualitative_analysis_ai, generate_qualitative_analysis_fallback,
    generate_quantitative_analysis_ai, generate_quantitative_analysis_fallback,
    ComprehensiveAnalysis, ProcessedQuantitativeOutput,
    QuantitativeInput,
};

pub struct AnalyticsService {
    db_manager: Arc<DatabaseManager>,
    google_ai_client: Option<GoogleAiClient>,
}

impl AnalyticsService {
    pub fn new(db_manager: Arc<DatabaseManager>) -> Self {
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
            .map_err(|e| anyhow::anyhow!("Invalid session ID format: {e}"))?;

        // Get session data
        let session = session_repo
            .get_by_id(&session_uuid)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Session not found: {session_id}"))?;

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
}
