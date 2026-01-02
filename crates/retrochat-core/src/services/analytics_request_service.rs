use std::sync::Arc;

use crate::database::{
    AnalyticsRepository, AnalyticsRequestRepository, ChatSessionRepository, DatabaseManager,
};
use crate::models::{Analytics, AnalyticsRequest, OperationStatus};
use crate::services::analytics_service::AnalyticsService;
use crate::services::google_ai::GoogleAiClient;
use crate::services::llm::LlmClient;

pub struct AnalyticsRequestService {
    analytics_service: AnalyticsService,
    request_repo: AnalyticsRequestRepository,
    db_manager: Arc<DatabaseManager>,
}

impl AnalyticsRequestService {
    /// Backward compatibility: Create service with GoogleAiClient
    pub fn new(db_manager: Arc<DatabaseManager>, google_ai_client: GoogleAiClient) -> Self {
        let request_repo = AnalyticsRequestRepository::new(db_manager.clone());
        let analytics_service =
            AnalyticsService::new(db_manager.clone()).with_google_ai(google_ai_client);

        Self {
            analytics_service,
            request_repo,
            db_manager,
        }
    }

    /// Create service with generic LLM client
    pub fn new_with_llm(db_manager: Arc<DatabaseManager>, llm_client: Arc<dyn LlmClient>) -> Self {
        let request_repo = AnalyticsRequestRepository::new(db_manager.clone());
        let analytics_service =
            AnalyticsService::new(db_manager.clone()).with_llm_client(llm_client);

        Self {
            analytics_service,
            request_repo,
            db_manager,
        }
    }

    pub async fn create_analysis_request(
        &self,
        session_id: String,
        created_by: Option<String>,
        custom_prompt: Option<String>,
    ) -> Result<AnalyticsRequest, Box<dyn std::error::Error + Send + Sync>> {
        // Check if there's already an active request for this session
        let existing_requests = self.request_repo.find_by_session_id(&session_id).await?;
        for existing_request in &existing_requests {
            match existing_request.status {
                OperationStatus::Pending | OperationStatus::Running => {
                    return Err(format!(
                        "Session {} already has an active analysis request ({}). Please wait for it to complete or cancel it first.",
                        session_id, existing_request.id
                    ).into());
                }
                _ => {} // Allow creating new requests if existing ones are completed/failed/cancelled
            }
        }

        // Dirty check: Check if session has been updated since last completed analysis
        if custom_prompt.is_none() {
            // Only perform dirty check if there's no custom prompt
            // (custom prompts should always create new requests)
            if let Some(latest_completed) = existing_requests
                .iter()
                .filter(|r| matches!(r.status, OperationStatus::Completed))
                .max_by_key(|r| r.completed_at.as_ref())
            {
                // Get the session to check its updated_at timestamp
                let session_repo = ChatSessionRepository::new(&self.db_manager);
                if let Ok(Some(session)) = session_repo
                    .get_by_id(
                        &uuid::Uuid::parse_str(&session_id)
                            .map_err(|e| format!("Invalid session ID: {e}"))?,
                    )
                    .await
                {
                    if let Some(completed_at) = latest_completed.completed_at {
                        // If session hasn't been updated since the last analysis, return existing request
                        if session.updated_at <= completed_at {
                            tracing::info!(
                                session_id = %session_id,
                                last_analysis = %completed_at,
                                session_updated = %session.updated_at,
                                "Session unchanged since last analysis - using cached results"
                            );
                            return Err(format!(
                                "Session {session_id} has not been modified since last analysis (completed at {completed_at}). Use 'retrochat analytics show {session_id}' to view cached results."
                            ).into());
                        }
                    }
                }
            }
        }

        let request = AnalyticsRequest::new(session_id, created_by, custom_prompt);

        self.request_repo.create(&request).await?;

        Ok(request)
    }

    pub async fn execute_analysis(
        &self,
        request_id: String,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        // Get the request from database
        let mut request = self
            .request_repo
            .find_by_id(&request_id)
            .await?
            .ok_or("Request not found")?;

        // Check if request is already running or completed
        match request.status {
            OperationStatus::Running => {
                return Err("Request is already running".into());
            }
            OperationStatus::Completed => {
                return Err("Request is already completed".into());
            }
            _ => {}
        }

        // Mark request as running
        request.mark_running();
        self.request_repo.update(&request).await?;

        // Perform the analysis synchronously (blocking for CLI, but TUI will handle async)
        match self.perform_analysis(&request).await {
            Ok(analysis) => {
                // Mark request as completed
                // Note: analysis results are now stored via analytics_service
                request.mark_completed();
                self.request_repo.update(&request).await?;

                Ok(analysis.session_id)
            }
            Err(e) => {
                // Mark request as failed with error message
                request.mark_failed(e.to_string());
                self.request_repo.update(&request).await?;

                Err(e)
            }
        }
    }

    pub async fn cancel_analysis(
        &self,
        request_id: String,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut request = self
            .request_repo
            .find_by_id(&request_id)
            .await?
            .ok_or("Request not found")?;

        // Only allow cancelling pending or running requests
        match request.status {
            OperationStatus::Pending | OperationStatus::Running => {
                request.mark_cancelled();
                self.request_repo.update(&request).await?;

                Ok(())
            }
            _ => Err(format!("Cannot cancel request with status: {:?}", request.status).into()),
        }
    }

    pub async fn get_analysis_status(
        &self,
        request_id: String,
    ) -> Result<AnalyticsRequest, Box<dyn std::error::Error + Send + Sync>> {
        self.request_repo
            .find_by_id(&request_id)
            .await?
            .ok_or("Request not found".into())
    }

    pub async fn get_analysis_result(
        &self,
        request_id: String,
    ) -> Result<Option<Analytics>, Box<dyn std::error::Error + Send + Sync>> {
        // Get the request to check its status
        let request = self
            .request_repo
            .find_by_id(&request_id)
            .await?
            .ok_or("Request not found")?;

        // Only return result if the request is completed
        if !matches!(request.status, OperationStatus::Completed) {
            return Ok(None);
        }

        // Try to load from database first
        let analytics_repo = AnalyticsRepository::new(&self.db_manager);
        if let Some(analytics) = analytics_repo
            .get_analytics_by_request_id(&request_id)
            .await
            .map_err(|e| format!("Failed to load analytics from database: {e}"))?
        {
            return Ok(Some(analytics));
        }

        // If not found in database, regenerate it on-demand
        tracing::info!("Analysis not found in database, regenerating...");
        match self
            .analytics_service
            .analyze_session(&request.session_id, Some(request_id.clone()))
            .await
        {
            Ok(mut analytics) => {
                // Save the regenerated analytics
                analytics.analytics_request_id = request_id.clone();
                // Keep analysis_duration_ms as None since we don't track regeneration time
                analytics_repo
                    .save_analytics(&analytics)
                    .await
                    .map_err(|e| format!("Failed to save regenerated analytics: {e}"))?;
                Ok(Some(analytics))
            }
            Err(e) => {
                tracing::warn!(error = %e, "Failed to regenerate analysis result");
                Ok(None)
            }
        }
    }

    pub async fn list_analyses(
        &self,
        session_id: Option<String>,
        limit: Option<usize>,
    ) -> Result<Vec<AnalyticsRequest>, Box<dyn std::error::Error + Send + Sync>> {
        match session_id {
            Some(session_id) => self.request_repo.find_by_session_id(&session_id).await,
            None => self.request_repo.find_recent(limit).await,
        }
    }

    pub async fn get_active_analyses(
        &self,
    ) -> Result<Vec<AnalyticsRequest>, Box<dyn std::error::Error + Send + Sync>> {
        self.request_repo.find_active_requests().await
    }

    pub async fn cancel_all_active_analyses(
        &self,
    ) -> Result<u32, Box<dyn std::error::Error + Send + Sync>> {
        let active_requests = self.get_active_analyses().await?;
        let mut cancelled_count = 0;

        for request in active_requests {
            if let Ok(()) = self.cancel_analysis(request.id).await {
                cancelled_count += 1;
            }
        }

        Ok(cancelled_count)
    }

    pub async fn cleanup_old_analyses(
        &self,
        days_old: u32,
    ) -> Result<u64, Box<dyn std::error::Error + Send + Sync>> {
        let cutoff_date = chrono::Utc::now() - chrono::Duration::days(days_old as i64);

        let requests_deleted = self
            .request_repo
            .delete_completed_before(cutoff_date)
            .await?;

        Ok(requests_deleted)
    }

    async fn perform_analysis(
        &self,
        request: &AnalyticsRequest,
    ) -> Result<Analytics, Box<dyn std::error::Error + Send + Sync>> {
        let start_time = std::time::Instant::now();

        // Use analytics service to generate analysis
        let mut analytics = self
            .analytics_service
            .analyze_session(&request.session_id, Some(request.id.clone()))
            .await?;

        // Save analysis to database with timing info
        let analysis_duration_ms = start_time.elapsed().as_millis() as i64;
        analytics.analytics_request_id = request.id.clone();
        analytics.analysis_duration_ms = Some(analysis_duration_ms);

        let analytics_repo = AnalyticsRepository::new(&self.db_manager);
        analytics_repo
            .save_analytics(&analytics)
            .await
            .map_err(|e| format!("Failed to save analytics: {e}"))?;

        Ok(analytics)
    }
}

/// A cleanup handler that automatically cancels running analyze requests when dropped.
/// This is useful for ensuring cleanup when the CLI exits or crashes.
pub struct AnalyticsRequestCleanupHandler {
    service: Arc<AnalyticsRequestService>,
    runtime: Arc<tokio::runtime::Runtime>,
}

impl AnalyticsRequestCleanupHandler {
    pub fn new(
        service: Arc<AnalyticsRequestService>,
        runtime: Arc<tokio::runtime::Runtime>,
    ) -> Self {
        Self { service, runtime }
    }
}

impl Drop for AnalyticsRequestCleanupHandler {
    fn drop(&mut self) {
        // Cancel all active analyze requests when the handler is dropped
        let service = self.service.clone();
        self.runtime.block_on(async move {
            match service.cancel_all_active_analyses().await {
                Ok(count) if count > 0 => {
                    tracing::info!(
                        count = count,
                        "Cancelled running analyze requests due to CLI exit"
                    );
                }
                Ok(_) => {
                    // No active requests to cancel
                }
                Err(e) => {
                    tracing::warn!(error = %e, "Failed to cancel active analyze requests");
                }
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::Database;
    use crate::services::google_ai::GoogleAiConfig;

    #[tokio::test]
    async fn test_create_analysis_request() {
        let database = Database::new_in_memory().await.unwrap();
        database.initialize().await.unwrap();

        // Create a test project first (required for foreign key constraint)
        let project_repo = crate::database::ProjectRepository::new(&database.manager);
        let test_project = crate::models::Project::new("test_project".to_string());
        project_repo.create(&test_project).await.unwrap();

        // Create a test session
        let session_repo = crate::database::ChatSessionRepository::new(&database.manager);
        let test_session = crate::models::ChatSession::new(
            crate::models::Provider::ClaudeCode,
            "/test/chat.jsonl".to_string(),
            "test_hash".to_string(),
            chrono::Utc::now(),
        )
        .with_project("test_project".to_string());
        session_repo.create(&test_session).await.unwrap();

        let service = AnalyticsRequestService::new(
            Arc::new(database.manager),
            GoogleAiClient::new(GoogleAiConfig::new("test-api-key".to_string())).unwrap(),
        );

        let session_id = test_session.id.to_string();
        let request = service
            .create_analysis_request(session_id.clone(), Some("test_user".to_string()), None)
            .await
            .unwrap();

        assert_eq!(request.session_id, session_id);
        assert_eq!(request.status, OperationStatus::Pending);
    }

    #[tokio::test]
    async fn test_get_analysis_status() {
        let database = Database::new_in_memory().await.unwrap();
        database.initialize().await.unwrap();

        // Create a test project first (required for foreign key constraint)
        let project_repo = crate::database::ProjectRepository::new(&database.manager);
        let test_project = crate::models::Project::new("test_project2".to_string());
        project_repo.create(&test_project).await.unwrap();

        // Create a test session
        let session_repo = crate::database::ChatSessionRepository::new(&database.manager);
        let test_session = crate::models::ChatSession::new(
            crate::models::Provider::ClaudeCode,
            "/test/chat2.jsonl".to_string(),
            "test_hash2".to_string(),
            chrono::Utc::now(),
        )
        .with_project("test_project2".to_string());
        session_repo.create(&test_session).await.unwrap();

        let service = AnalyticsRequestService::new(
            Arc::new(database.manager),
            GoogleAiClient::new(GoogleAiConfig::new("test-api-key".to_string())).unwrap(),
        );

        let session_id = test_session.id.to_string();
        let request = service
            .create_analysis_request(session_id, Some("test_user".to_string()), None)
            .await
            .unwrap();

        let status = service
            .get_analysis_status(request.id.clone())
            .await
            .unwrap();
        assert_eq!(status.id, request.id);
        assert_eq!(status.status, OperationStatus::Pending);
    }

    #[tokio::test]
    async fn test_dirty_check_prevents_duplicate_analysis() {
        let database = Database::new_in_memory().await.unwrap();
        database.initialize().await.unwrap();

        // Create a test project
        let project_repo = crate::database::ProjectRepository::new(&database.manager);
        let test_project = crate::models::Project::new("test_project3".to_string());
        project_repo.create(&test_project).await.unwrap();

        // Create a test session
        let session_repo = crate::database::ChatSessionRepository::new(&database.manager);
        let test_session = crate::models::ChatSession::new(
            crate::models::Provider::ClaudeCode,
            "/test/chat3.jsonl".to_string(),
            "test_hash3".to_string(),
            chrono::Utc::now(),
        )
        .with_project("test_project3".to_string());
        session_repo.create(&test_session).await.unwrap();

        let service = AnalyticsRequestService::new(
            Arc::new(database.manager.clone()),
            GoogleAiClient::new(GoogleAiConfig::new("test-api-key".to_string())).unwrap(),
        );

        let session_id = test_session.id.to_string();

        // Create first request
        let first_request = service
            .create_analysis_request(session_id.clone(), None, None)
            .await
            .unwrap();

        // Mark it as completed
        let mut completed_request = first_request.clone();
        completed_request.mark_completed();
        let request_repo = AnalyticsRequestRepository::new(Arc::new(database.manager.clone()));
        request_repo.update(&completed_request).await.unwrap();

        // Wait a bit to ensure timestamp difference
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        // Try to create second request without updating the session
        // This should fail with dirty check error
        let result = service
            .create_analysis_request(session_id.clone(), None, None)
            .await;

        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("has not been modified since last analysis"));
    }

    #[tokio::test]
    async fn test_dirty_check_bypassed_with_custom_prompt() {
        let database = Database::new_in_memory().await.unwrap();
        database.initialize().await.unwrap();

        // Create a test project
        let project_repo = crate::database::ProjectRepository::new(&database.manager);
        let test_project = crate::models::Project::new("test_project4".to_string());
        project_repo.create(&test_project).await.unwrap();

        // Create a test session
        let session_repo = crate::database::ChatSessionRepository::new(&database.manager);
        let test_session = crate::models::ChatSession::new(
            crate::models::Provider::ClaudeCode,
            "/test/chat4.jsonl".to_string(),
            "test_hash4".to_string(),
            chrono::Utc::now(),
        )
        .with_project("test_project4".to_string());
        session_repo.create(&test_session).await.unwrap();

        let service = AnalyticsRequestService::new(
            Arc::new(database.manager.clone()),
            GoogleAiClient::new(GoogleAiConfig::new("test-api-key".to_string())).unwrap(),
        );

        let session_id = test_session.id.to_string();

        // Create first request
        let first_request = service
            .create_analysis_request(session_id.clone(), None, None)
            .await
            .unwrap();

        // Mark it as completed
        let mut completed_request = first_request.clone();
        completed_request.mark_completed();
        let request_repo = AnalyticsRequestRepository::new(Arc::new(database.manager.clone()));
        request_repo.update(&completed_request).await.unwrap();

        // Create second request with custom prompt (should bypass dirty check)
        let result = service
            .create_analysis_request(
                session_id.clone(),
                None,
                Some("Custom analysis prompt".to_string()),
            )
            .await;

        assert!(result.is_ok());
        let second_request = result.unwrap();
        assert_eq!(
            second_request.custom_prompt,
            Some("Custom analysis prompt".to_string())
        );
    }
}
