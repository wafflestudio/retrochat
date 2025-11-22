use super::google_ai::GoogleAiClient;
use crate::database::{
    ChatSessionRepository, DatabaseManager, MessageRepository, ToolOperationRepository,
};
use anyhow::Result;
use std::sync::Arc;

// Import from analytics module
use super::analytics::{
    collect_qualitative_data, collect_quantitative_data, generate_qualitative_analysis_ai,
    generate_quantitative_analysis_ai,
};
use crate::models::{Analytics, Metrics};

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

    pub async fn analyze_session(
        &self,
        session_id: &str,
        analytics_request_id: Option<String>,
    ) -> Result<Analytics> {
        tracing::info!("Starting analysis for session: {}", session_id);

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

        // Generate analysis (requires AI client)
        let ai_client = self
            .google_ai_client
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("AI client is required for analysis"))?;

        let ai_qualitative_output =
            generate_qualitative_analysis_ai(&qualitative_input, ai_client, None).await?;

        let ai_quantitative_output =
            generate_quantitative_analysis_ai(&qualitative_input, ai_client, None).await?;

        // Build metrics from quantitative_input
        let metrics = Metrics {
            total_files_modified: quantitative_input.file_changes.total_files_modified,
            total_files_read: quantitative_input.file_changes.total_files_read,
            lines_added: quantitative_input.file_changes.lines_added,
            lines_removed: quantitative_input.file_changes.lines_removed,
            total_tokens_used: quantitative_input.token_metrics.total_tokens_used,
            session_duration_minutes: quantitative_input.time_metrics.total_session_time_minutes,
        };

        // Create Analytics directly
        Ok(Analytics::new(
            analytics_request_id.unwrap_or_else(|| "temp-request".to_string()),
            session_id.to_string(),
            ai_qualitative_output,
            ai_quantitative_output,
            metrics,
            None, // model_used - will be set later if available
            None, // analysis_duration_ms - will be set later
        ))
    }
}
