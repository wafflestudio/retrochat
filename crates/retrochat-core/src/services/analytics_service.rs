use crate::database::{
    ChatSessionRepository, DatabaseManager, MessageRepository, ToolOperationRepository,
};
use crate::services::llm::LlmProvider;
use anyhow::Result;
use std::sync::Arc;

// Import from analytics module
use super::analytics::{
    collect_qualitative_data, collect_quantitative_data, generate_qualitative_analysis_ai,
    generate_quantitative_analysis_ai,
};
use crate::models::Analytics;

pub struct AnalyticsService {
    db_manager: Arc<DatabaseManager>,
    llm_provider: Option<Arc<dyn LlmProvider>>,
}

impl AnalyticsService {
    pub fn new(db_manager: Arc<DatabaseManager>) -> Self {
        Self {
            db_manager,
            llm_provider: None,
        }
    }

    /// Set the LLM provider to use for analysis
    pub fn with_llm_provider(mut self, provider: Arc<dyn LlmProvider>) -> Self {
        self.llm_provider = Some(provider);
        self
    }

    /// Get the LLM provider, if configured
    pub fn llm_provider(&self) -> Option<&Arc<dyn LlmProvider>> {
        self.llm_provider.as_ref()
    }

    /// Get the model name being used, if a provider is configured
    pub fn model_name(&self) -> Option<&str> {
        self.llm_provider.as_ref().map(|p| p.model_name())
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
        let metric_quantitative_output =
            collect_quantitative_data(&session, &messages, &tool_operations).await?;
        let qualitative_input =
            collect_qualitative_data(&tool_operations, &messages, &session).await?;

        // Generate analysis (requires LLM provider)
        let llm_provider = self
            .llm_provider
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("LLM provider is required for analysis"))?
            .clone();

        let model_name = llm_provider.model_name().to_string();

        // Run qualitative and quantitative analysis in parallel
        // try_join! cancels remaining futures immediately if one fails
        let provider_for_qualitative = llm_provider.clone();
        let provider_for_quantitative = llm_provider;

        let (ai_qualitative_output, ai_quantitative_output) = tokio::try_join!(
            generate_qualitative_analysis_ai(&qualitative_input, provider_for_qualitative, None),
            generate_quantitative_analysis_ai(&qualitative_input, provider_for_quantitative, None)
        )?;

        // Create Analytics directly
        Ok(Analytics::new(
            analytics_request_id.unwrap_or_else(|| "temp-request".to_string()),
            session_id.to_string(),
            ai_qualitative_output,
            ai_quantitative_output,
            metric_quantitative_output,
            Some(model_name),
            None, // analysis_duration_ms - will be set later
        ))
    }
}
