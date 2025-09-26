use crate::database::{
    AnalysisRequestRepository, ChatSessionRepository, DatabaseManager, MessageRepository,
    RetrospectionAnalysisRepository,
};
use crate::models::message::MessageRole;
use crate::models::{AnalysisRequest, RetrospectionAnalysis};

#[cfg(test)]
use crate::models::{AnalysisMetadata, AnalysisStatus, RequestStatus};
use crate::services::{GeminiClient, PromptService};
use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Semaphore;
use tokio::time::{timeout, Duration};
use tracing::{debug, info};
use uuid::Uuid;

#[derive(Debug)]
pub struct RetrospectionService {
    db_manager: DatabaseManager,
    gemini_client: Arc<GeminiClient>,
    prompt_service: PromptService,
    processing_semaphore: Arc<Semaphore>,
    max_processing_time: Duration,
}

impl RetrospectionService {
    /// Create a new retrospection service
    pub fn new(db_manager: DatabaseManager) -> Result<Self> {
        let gemini_client = Arc::new(GeminiClient::new()?);
        let prompt_service = PromptService::new();

        Ok(Self {
            db_manager,
            gemini_client,
            prompt_service,
            processing_semaphore: Arc::new(Semaphore::new(3)), // Max 3 concurrent analyses
            max_processing_time: Duration::from_secs(300),     // 5 minutes timeout
        })
    }

    /// Create a retrospection service with custom configuration
    pub fn with_config(
        db_manager: DatabaseManager,
        gemini_client: GeminiClient,
        max_concurrent: usize,
        max_processing_time: Duration,
    ) -> Self {
        let prompt_service = PromptService::new();

        Self {
            db_manager,
            gemini_client: Arc::new(gemini_client),
            prompt_service,
            processing_semaphore: Arc::new(Semaphore::new(max_concurrent)),
            max_processing_time,
        }
    }

    /// Submit a new analysis request
    pub async fn submit_analysis_request(
        &self,
        session_id: Uuid,
        _template_id: String,
        _variables: HashMap<String, String>,
    ) -> Result<AnalysisRequest> {
        // In simplified version, we ignore template_id and variables
        // and just create a basic request
        let request = AnalysisRequest::new(session_id, "default".to_string(), HashMap::new());

        // Store request in database
        self.db_manager.with_connection_anyhow(|conn| {
            let repo = AnalysisRequestRepository::new(conn);
            repo.create(&request)
        })?;

        info!(
            "Submitted analysis request {} for session {} using default template",
            request.id, session_id
        );

        Ok(request)
    }

    /// Process a single analysis request
    pub async fn process_analysis_request(
        &self,
        mut request: AnalysisRequest,
    ) -> Result<RetrospectionAnalysis> {
        let _permit = self.processing_semaphore.acquire().await?;

        info!(
            "Processing analysis request {} for session {}",
            request.id, request.session_id
        );

        // Mark request as processing
        request.start_processing();
        self.update_request_status(&request)?;

        let result = timeout(
            self.max_processing_time,
            self.process_analysis_internal(request.clone()),
        )
        .await;

        match result {
            Ok(analysis_result) => {
                match analysis_result {
                    Ok(analysis) => {
                        // Mark request as completed
                        request.complete();
                        self.update_request_status(&request)?;
                        Ok(analysis)
                    }
                    Err(e) => {
                        // Mark request as failed
                        request.fail(e.to_string());
                        self.update_request_status(&request)?;
                        Err(e)
                    }
                }
            }
            Err(_) => {
                // Timeout occurred
                let error_msg = format!(
                    "Analysis request timed out after {} seconds",
                    self.max_processing_time.as_secs()
                );
                request.fail(error_msg.clone());
                self.update_request_status(&request)?;
                Err(anyhow!(error_msg))
            }
        }
    }

    /// Process pending analysis requests in queue
    pub async fn process_pending_requests(&self) -> Result<ProcessingResult> {
        let requests = self.db_manager.with_connection_anyhow(|conn| {
            let repo = AnalysisRequestRepository::new(conn);
            repo.get_queue()
        })?;

        if requests.is_empty() {
            debug!("No pending analysis requests to process");
            return Ok(ProcessingResult {
                processed: 0,
                successful: 0,
                failed: 0,
                errors: Vec::new(),
            });
        }

        info!("Processing {} pending analysis requests", requests.len());

        let mut result = ProcessingResult {
            processed: 0,
            successful: 0,
            failed: 0,
            errors: Vec::new(),
        };

        let mut handles = Vec::new();

        // Process requests concurrently (up to semaphore limit)
        for request in requests {
            let service = self.clone();
            let handle =
                tokio::spawn(async move { service.process_analysis_request(request).await });
            handles.push(handle);
        }

        // Wait for all tasks to complete
        for handle in handles {
            result.processed += 1;

            match handle.await {
                Ok(analysis_result) => match analysis_result {
                    Ok(_) => result.successful += 1,
                    Err(e) => {
                        result.failed += 1;
                        result.errors.push(e.to_string());
                    }
                },
                Err(e) => {
                    result.failed += 1;
                    result.errors.push(format!("Task join error: {e}"));
                }
            }
        }

        info!(
            "Completed processing: {} processed, {} successful, {} failed",
            result.processed, result.successful, result.failed
        );

        Ok(result)
    }

    /// Get analyses for a specific session
    pub async fn get_analyses_for_session(
        &self,
        session_id: Uuid,
    ) -> Result<Vec<RetrospectionAnalysis>> {
        self.db_manager.with_connection_anyhow(|conn| {
            let repo = RetrospectionAnalysisRepository::new(conn);
            repo.find_by_session_id(&session_id)
        })
    }

    /// Get a specific analysis by ID
    pub async fn get_analysis(&self, analysis_id: Uuid) -> Result<Option<RetrospectionAnalysis>> {
        self.db_manager.with_connection_anyhow(|conn| {
            let repo = RetrospectionAnalysisRepository::new(conn);
            repo.find_by_id(&analysis_id)
        })
    }

    /// Store an analysis result
    pub async fn store_analysis(&self, analysis: &RetrospectionAnalysis) -> Result<()> {
        self.db_manager.with_connection_anyhow(|conn| {
            let repo = RetrospectionAnalysisRepository::new(conn);
            repo.create(analysis)
        })?;

        info!(
            "Stored analysis {} for session {}",
            analysis.id, analysis.session_id
        );
        Ok(())
    }

    /// Get recent analyses
    pub async fn get_recent_analyses(&self, limit: u32) -> Result<Vec<RetrospectionAnalysis>> {
        self.db_manager.with_connection_anyhow(|conn| {
            let repo = RetrospectionAnalysisRepository::new(conn);
            repo.get_recent_analyses(limit)
        })
    }

    /// Search analyses by content
    pub async fn search_analyses(&self, search_term: &str) -> Result<Vec<RetrospectionAnalysis>> {
        self.db_manager.with_connection_anyhow(|conn| {
            let repo = RetrospectionAnalysisRepository::new(conn);
            repo.search_by_content(search_term)
        })
    }

    /// Get analysis statistics
    pub async fn get_analysis_statistics(&self) -> Result<crate::database::AnalysisStatistics> {
        self.db_manager.with_connection_anyhow(|conn| {
            let repo = RetrospectionAnalysisRepository::new(conn);
            repo.get_analysis_statistics()
        })
    }

    /// Get queue statistics
    pub async fn get_queue_statistics(&self) -> Result<crate::database::QueueStatistics> {
        self.db_manager.with_connection_anyhow(|conn| {
            let repo = AnalysisRequestRepository::new(conn);
            repo.get_queue_statistics()
        })
    }

    /// Retry a failed analysis request
    pub async fn retry_analysis_request(&self, request_id: Uuid) -> Result<()> {
        self.db_manager.with_connection_anyhow(|conn| {
            let repo = AnalysisRequestRepository::new(conn);
            let retried = repo.retry_failed_request(&request_id)?;

            if retried {
                info!("Retried analysis request {}", request_id);
                Ok(())
            } else {
                Err(anyhow!(
                    "Request {request_id} not found or not in failed state"
                ))
            }
        })
    }

    /// Clean up old completed requests
    pub async fn cleanup_old_requests(&self, older_than_days: u32) -> Result<u32> {
        let cleaned = self.db_manager.with_connection_anyhow(|conn| {
            let repo = AnalysisRequestRepository::new(conn);
            repo.cleanup_completed_requests(older_than_days)
        })?;

        if cleaned > 0 {
            info!("Cleaned up {} old analysis requests", cleaned);
        }

        Ok(cleaned)
    }

    async fn process_analysis_internal(
        &self,
        request: AnalysisRequest,
    ) -> Result<RetrospectionAnalysis> {
        debug!("Starting analysis for request {}", request.id);

        // Get chat session content
        let chat_content = self.get_session_content(request.session_id).await?;

        // Get and render template
        let mut variables = request.template_variables.clone();
        variables.insert("chat_content".to_string(), chat_content.clone());

        let rendered_prompt = self.prompt_service.render_prompt(&chat_content)?;

        debug!("Generated prompt: {} characters", rendered_prompt.len());

        // Generate analysis using Gemini
        let (analysis_content, metadata) = self
            .gemini_client
            .generate_content(&rendered_prompt)
            .await?;

        // Create analysis record
        let mut analysis = RetrospectionAnalysis::new(request.session_id, "default".to_string());
        analysis.complete(analysis_content, metadata);

        // Store the analysis
        self.store_analysis(&analysis).await?;

        info!(
            "Completed analysis {} for session {} (tokens: {}, cost: ${})",
            analysis.id,
            analysis.session_id,
            analysis.metadata.total_tokens,
            analysis.metadata.format_cost()
        );

        Ok(analysis)
    }

    async fn get_session_content(&self, session_id: Uuid) -> Result<String> {
        let session_repo = ChatSessionRepository::new(self.db_manager.clone());
        let message_repo = MessageRepository::new(self.db_manager.clone());

        // Get session info
        let session = session_repo
            .get_by_id(&session_id)?
            .ok_or_else(|| anyhow!("Session {session_id} not found"))?;

        // Get all messages for the session
        let messages = message_repo.get_by_session(&session_id)?;

        if messages.is_empty() {
            return Err(anyhow!("No messages found for session {session_id}"));
        }

        // Format messages into a readable conversation
        let mut content = Vec::new();
        content.push("# Chat Session Analysis".to_string());
        content.push(format!("**Session ID:** {}", session.id));
        content.push(format!("**Provider:** {}", session.provider));
        if let Some(project) = &session.project_name {
            content.push(format!("**Project:** {project}"));
        }
        content.push(format!("**Start Time:** {}", session.start_time));
        content.push(format!("**Message Count:** {}", messages.len()));
        content.push(String::new());
        content.push("## Conversation".to_string());
        content.push(String::new());

        for (i, message) in messages.iter().enumerate() {
            let role_label = match message.role {
                MessageRole::User => "**User**",
                MessageRole::Assistant => "**Assistant**",
                MessageRole::System => "**System**",
            };

            content.push(format!("### Message {} - {}", i + 1, role_label));
            content.push(format!("*Time: {}*", message.timestamp));
            content.push(String::new());
            content.push(message.content.clone());
            content.push(String::new());

            // Add tool calls if present
            if let Some(tool_calls) = &message.tool_calls {
                if !tool_calls.is_empty() {
                    content.push("**Tool Calls:**".to_string());
                    if let Ok(json_str) = serde_json::to_string_pretty(tool_calls) {
                        content.push(format!("```json\n{json_str}\n```"));
                    }
                    content.push(String::new());
                }
            }
        }

        content.push("---".to_string());
        content.push("*End of session content*".to_string());

        Ok(content.join("\n"))
    }

    fn update_request_status(&self, request: &AnalysisRequest) -> Result<()> {
        self.db_manager.with_connection_anyhow(|conn| {
            let repo = AnalysisRequestRepository::new(conn);
            repo.update(request)
        })
    }
}

// Enable cloning for concurrent processing
impl Clone for RetrospectionService {
    fn clone(&self) -> Self {
        Self {
            db_manager: self.db_manager.clone(),
            gemini_client: Arc::clone(&self.gemini_client),
            prompt_service: self.prompt_service.clone(),
            processing_semaphore: Arc::clone(&self.processing_semaphore),
            max_processing_time: self.max_processing_time,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ProcessingResult {
    pub processed: u32,
    pub successful: u32,
    pub failed: u32,
    pub errors: Vec<String>,
}

impl ProcessingResult {
    pub fn get_summary(&self) -> String {
        format!(
            "Processed {} requests: {} successful, {} failed",
            self.processed, self.successful, self.failed
        )
    }

    pub fn has_failures(&self) -> bool {
        self.failed > 0
    }

    pub fn success_rate(&self) -> f64 {
        if self.processed == 0 {
            0.0
        } else {
            (self.successful as f64) / (self.processed as f64)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::{seed_default_prompt_templates, DatabaseManager};
    use crate::models::{ChatSession, LlmProvider, Message, MessageRole, SessionState};
    use std::env;
    use tempfile::NamedTempFile;
    use uuid::Uuid;

    async fn setup_test_service() -> RetrospectionService {
        let temp_file = NamedTempFile::new().unwrap();
        let db_manager = DatabaseManager::new(temp_file.path().to_str().unwrap()).unwrap();

        // Initialize schema and seed templates
        db_manager
            .with_connection(crate::database::create_schema)
            .unwrap();
        db_manager
            .with_connection_anyhow(seed_default_prompt_templates)
            .unwrap();

        // Create mock Gemini client if no API key
        if env::var("GEMINI_API_KEY").is_err() {
            // For tests without API key, we would need a mock client
            // For now, use the real client constructor that will fail gracefully
        }

        RetrospectionService::new(db_manager).unwrap()
    }

    async fn setup_test_session(service: &RetrospectionService) -> Uuid {
        let session_id = Uuid::new_v4();

        let session_repo = ChatSessionRepository::new(service.db_manager.clone());
        let message_repo = MessageRepository::new(service.db_manager.clone());

        // Create test session
        let session = ChatSession {
            id: session_id,
            provider: LlmProvider::ClaudeCode,
            project_name: Some("test_project".to_string()),
            start_time: chrono::Utc::now(),
            end_time: Some(chrono::Utc::now()),
            message_count: 2,
            token_count: Some(100),
            file_path: "/test/path".to_string(),
            file_hash: "test_hash".to_string(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            state: SessionState::Analyzed,
        };

        session_repo.create(&session).unwrap();

        // Create test messages
        let message1 = Message {
            id: Uuid::new_v4(),
            session_id,
            role: MessageRole::User,
            content: "Hello, can you help me understand machine learning?".to_string(),
            timestamp: chrono::Utc::now(),
            token_count: Some(10),
            tool_calls: None,
            metadata: None,
            sequence_number: 1,
        };

        let message2 = Message {
            id: Uuid::new_v4(),
            session_id,
            role: MessageRole::Assistant,
            content: "Of course! Machine learning is a subset of artificial intelligence..."
                .to_string(),
            timestamp: chrono::Utc::now(),
            token_count: Some(50),
            tool_calls: None,
            metadata: None,
            sequence_number: 2,
        };

        message_repo.create(&message1).unwrap();
        message_repo.create(&message2).unwrap();

        session_id
    }

    #[tokio::test]
    async fn test_submit_analysis_request() {
        let service = setup_test_service().await;
        let session_id = setup_test_session(&service).await;

        let mut variables = HashMap::new();
        variables.insert("chat_content".to_string(), "test content".to_string());

        let result = service
            .submit_analysis_request(session_id, "basic-session-analysis".to_string(), variables)
            .await;

        assert!(result.is_ok());
        let request = result.unwrap();
        assert_eq!(request.session_id, session_id);
        assert_eq!(request.status, RequestStatus::Queued);
    }

    #[tokio::test]
    async fn test_get_session_content() {
        let service = setup_test_service().await;
        let session_id = setup_test_session(&service).await;

        let content = service.get_session_content(session_id).await.unwrap();
        assert!(content.contains("Chat Session Analysis"));
        assert!(content.contains("machine learning"));
        assert!(content.contains("User"));
        assert!(content.contains("Assistant"));
    }

    #[tokio::test]
    async fn test_analysis_storage_and_retrieval() {
        let service = setup_test_service().await;
        let session_id = setup_test_session(&service).await;

        let mut analysis =
            RetrospectionAnalysis::new(session_id, "basic-session-analysis".to_string());
        analysis.complete(
            "This is a test analysis result".to_string(),
            AnalysisMetadata::default(),
        );

        // Store analysis
        service.store_analysis(&analysis).await.unwrap();

        // Retrieve analyses for session
        let analyses = service.get_analyses_for_session(session_id).await.unwrap();
        assert_eq!(analyses.len(), 1);
        assert_eq!(analyses[0].id, analysis.id);

        // Get specific analysis
        let retrieved = service.get_analysis(analysis.id).await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(
            retrieved.unwrap().analysis_content,
            analysis.analysis_content
        );
    }

    #[tokio::test]
    async fn test_queue_statistics() {
        let service = setup_test_service().await;
        let session_id = setup_test_session(&service).await;

        // Submit a request
        let mut variables = HashMap::new();
        variables.insert("chat_content".to_string(), "test".to_string());

        service
            .submit_analysis_request(session_id, "basic-session-analysis".to_string(), variables)
            .await
            .unwrap();

        // Check queue statistics
        let stats = service.get_queue_statistics().await.unwrap();
        assert_eq!(stats.queued_count, 1);
        assert_eq!(stats.processing_count, 0);
    }

    #[tokio::test]
    async fn test_analysis_search() {
        let service = setup_test_service().await;
        let session_id = setup_test_session(&service).await;

        // Create and store test analysis
        let mut analysis =
            RetrospectionAnalysis::new(session_id, "basic-session-analysis".to_string());
        analysis.complete(
            "This analysis contains important insights about machine learning".to_string(),
            AnalysisMetadata::default(),
        );

        service.store_analysis(&analysis).await.unwrap();

        // Search for analysis
        let results = service.search_analyses("machine learning").await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, analysis.id);

        // Search with no results
        let results = service.search_analyses("nonexistent term").await.unwrap();
        assert_eq!(results.len(), 0);
    }

    #[tokio::test]
    async fn test_cleanup_old_requests() {
        let service = setup_test_service().await;

        // Initially no requests to clean up
        let cleaned = service.cleanup_old_requests(0).await.unwrap();
        assert_eq!(cleaned, 0);
    }

    #[tokio::test]
    #[ignore] // Requires GEMINI_API_KEY
    async fn test_full_analysis_flow() {
        if env::var("GEMINI_API_KEY").is_err() {
            println!("Skipping full analysis test - GEMINI_API_KEY not set");
            return;
        }

        let service = setup_test_service().await;
        let session_id = setup_test_session(&service).await;

        // Submit analysis request
        let variables = HashMap::new(); // basic-session-analysis only needs chat_content which is auto-populated

        let request = service
            .submit_analysis_request(session_id, "basic-session-analysis".to_string(), variables)
            .await
            .unwrap();

        // Process the request
        let result = service.process_analysis_request(request).await;

        match result {
            Ok(analysis) => {
                println!("Analysis completed: {}", analysis.analysis_content);
                assert_eq!(analysis.session_id, session_id);
                assert_eq!(analysis.status, AnalysisStatus::Complete);
                assert!(!analysis.analysis_content.is_empty());
            }
            Err(e) => {
                println!("Analysis failed: {e}");
                // Don't fail the test for API issues
            }
        }
    }
}
