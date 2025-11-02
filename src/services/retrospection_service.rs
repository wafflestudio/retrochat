use std::sync::Arc;

use crate::database::{DatabaseManager, RetrospectRequestRepository};
use crate::models::{OperationStatus, RetrospectRequest, Retrospection};
use crate::services::analytics_service::AnalyticsService;
use crate::services::google_ai::GoogleAiClient;

pub struct RetrospectionService {
    analytics_service: AnalyticsService,
    request_repo: RetrospectRequestRepository,
}

impl RetrospectionService {
    pub fn new(db_manager: Arc<DatabaseManager>, google_ai_client: GoogleAiClient) -> Self {
        let request_repo = RetrospectRequestRepository::new(db_manager.clone());
        let analytics_service = AnalyticsService::new(db_manager).with_google_ai(google_ai_client);

        Self {
            analytics_service,
            request_repo,
        }
    }

    pub async fn create_analysis_request(
        &self,
        session_id: String,
        created_by: Option<String>,
        custom_prompt: Option<String>,
    ) -> Result<RetrospectRequest, Box<dyn std::error::Error + Send + Sync>> {
        // Check if there's already an active request for this session
        let existing_requests = self.request_repo.find_by_session_id(&session_id).await?;
        for existing_request in existing_requests {
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

        let request = RetrospectRequest::new(session_id, created_by, custom_prompt);

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
            Ok(retrospection) => {
                // Mark request as completed
                // Note: retrospection results are now stored via analytics_service
                request.mark_completed();
                self.request_repo.update(&request).await?;

                Ok(retrospection.id)
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
    ) -> Result<RetrospectRequest, Box<dyn std::error::Error + Send + Sync>> {
        self.request_repo
            .find_by_id(&request_id)
            .await?
            .ok_or("Request not found".into())
    }

    pub async fn get_analysis_result(
        &self,
        request_id: String,
    ) -> Result<Option<Retrospection>, Box<dyn std::error::Error + Send + Sync>> {
        // Get the request to check its status
        let request = self.request_repo.find_by_id(&request_id).await?
            .ok_or("Request not found")?;

        // Only return result if the request is completed
        if !matches!(request.status, OperationStatus::Completed) {
            return Ok(None);
        }

        // Note: Retrospection data is now managed by analytics service
        // For now, if analysis was completed, we can regenerate it on-demand
        // This could be optimized later by storing results in analytics_repo
        match self.analytics_service.analyze_session_comprehensive(&request.session_id).await {
            Ok(analysis) => {
                let retrospection = Retrospection::from_comprehensive_analysis(
                    request_id,
                    analysis,
                    Some("gemini-pro".to_string()),
                    None,
                );
                Ok(Some(retrospection))
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
    ) -> Result<Vec<RetrospectRequest>, Box<dyn std::error::Error + Send + Sync>> {
        match session_id {
            Some(session_id) => self.request_repo.find_by_session_id(&session_id).await,
            None => self.request_repo.find_recent(limit).await,
        }
    }

    pub async fn get_active_analyses(
        &self,
    ) -> Result<Vec<RetrospectRequest>, Box<dyn std::error::Error + Send + Sync>> {
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

        // Clean up old completed requests
        // Note: retrospection data is now managed by analytics_service
        let requests_deleted = self
            .request_repo
            .delete_completed_before(cutoff_date)
            .await?;

        Ok(requests_deleted)
    }

    async fn perform_analysis(
        &self,
        request: &RetrospectRequest,
    ) -> Result<Retrospection, Box<dyn std::error::Error + Send + Sync>> {
        // Use analytics service to generate comprehensive analysis
        let start_time = std::time::Instant::now();
        let analysis = self
            .analytics_service
            .analyze_session_comprehensive(&request.session_id)
            .await?;
        let analysis_duration_ms = start_time.elapsed().as_millis() as i64;

        // Convert to retrospection
        let retrospection = Retrospection::from_comprehensive_analysis(
            request.id.clone(),
            analysis,
            Some("gemini-pro".to_string()),
            Some(analysis_duration_ms),
        );

        Ok(retrospection)
    }
}

/// A cleanup handler that automatically cancels running retrospection requests when dropped.
/// This is useful for ensuring cleanup when the CLI exits or crashes.
pub struct RetrospectionCleanupHandler {
    service: Arc<RetrospectionService>,
    runtime: Arc<tokio::runtime::Runtime>,
}

impl RetrospectionCleanupHandler {
    pub fn new(service: Arc<RetrospectionService>, runtime: Arc<tokio::runtime::Runtime>) -> Self {
        Self { service, runtime }
    }
}

impl Drop for RetrospectionCleanupHandler {
    fn drop(&mut self) {
        // Cancel all active retrospection requests when the handler is dropped
        let service = self.service.clone();
        self.runtime.block_on(async move {
            match service.cancel_all_active_analyses().await {
                Ok(count) if count > 0 => {
                    tracing::info!(
                        count = count,
                        "Cancelled running retrospection requests due to CLI exit"
                    );
                }
                Ok(_) => {
                    // No active requests to cancel
                }
                Err(e) => {
                    tracing::warn!(error = %e, "Failed to cancel active retrospection requests");
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

        let service = RetrospectionService::new(
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

        let service = RetrospectionService::new(
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
}
