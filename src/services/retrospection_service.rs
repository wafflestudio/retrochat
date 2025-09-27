use std::sync::Arc;

use crate::database::{DatabaseManager, RetrospectRequestRepository, RetrospectionRepository};
use crate::models::{RetrospectRequest, Retrospection, RetrospectionAnalysisType, OperationStatus};
use crate::services::google_ai::{GoogleAiClient, GoogleAiConfig};
use crate::services::background::BackgroundOperationManager;

pub struct RetrospectionService {
    db_manager: Arc<DatabaseManager>,
    google_ai_client: GoogleAiClient,
    operation_manager: BackgroundOperationManager,
    request_repo: RetrospectRequestRepository,
    retrospection_repo: RetrospectionRepository,
}

impl RetrospectionService {
    pub fn new(
        db_manager: Arc<DatabaseManager>,
        google_ai_client: GoogleAiClient,
    ) -> Self {
        let operation_manager = BackgroundOperationManager::new(db_manager.clone());
        let request_repo = RetrospectRequestRepository::new(db_manager.clone());
        let retrospection_repo = RetrospectionRepository::new(db_manager.clone());

        Self {
            db_manager,
            google_ai_client,
            operation_manager,
            request_repo,
            retrospection_repo,
        }
    }

    pub async fn create_analysis_request(
        &self,
        session_id: String,
        analysis_type: RetrospectionAnalysisType,
        created_by: Option<String>,
        custom_prompt: Option<String>,
    ) -> Result<RetrospectRequest, Box<dyn std::error::Error + Send + Sync>> {
        let request = RetrospectRequest::new(
            session_id,
            analysis_type,
            created_by,
            custom_prompt,
        );

        self.request_repo.create(&request).await?;

        Ok(request)
    }

    pub async fn execute_analysis(
        &self,
        request_id: String,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        // Get the request from database
        let mut request = self.request_repo.find_by_id(&request_id).await?
            .ok_or("Request not found")?;

        // Check if request is already running or completed
        match request.status {
            OperationStatus::Running => {
                return Err("Request is already running".into());
            }
            OperationStatus::Completed => {
                return Err("Request is already completed".into());
            }
            OperationStatus::Cancelled => {
                return Err("Request was cancelled".into());
            }
            _ => {}
        }

        // Mark request as running
        request.mark_running();
        self.request_repo.update(&request).await?;

        // Start background operation
        let operation_id = self.operation_manager.start_operation(
            format!("retrospection-{}", request_id),
            "Retrospection Analysis".to_string(),
            None, // created_by
        ).await?;

        // In a real implementation, this would be done in a background task
        // For now, we'll do it synchronously but with proper error handling
        match self.perform_analysis(&request).await {
            Ok(retrospection) => {
                // Save the retrospection result
                self.retrospection_repo.create(&retrospection).await?;

                // Mark request as completed
                request.mark_completed();
                self.request_repo.update(&request).await?;

                // Complete the operation
                self.operation_manager.complete_operation(&operation_id, true, None).await?;

                Ok(retrospection.id)
            }
            Err(e) => {
                // Mark request as failed with error message
                request.mark_failed(e.to_string());
                self.request_repo.update(&request).await?;

                // Fail the operation
                self.operation_manager.fail_operation(&operation_id, e.to_string(), None).await?;

                Err(e)
            }
        }
    }

    pub async fn cancel_analysis(
        &self,
        request_id: String,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut request = self.request_repo.find_by_id(&request_id).await?
            .ok_or("Request not found")?;

        // Only allow cancelling pending or running requests
        match request.status {
            OperationStatus::Pending | OperationStatus::Running => {
                request.mark_cancelled();
                self.request_repo.update(&request).await?;

                // Cancel the background operation if it exists
                let operation_id = format!("retrospection-{}", request_id);
                if let Ok(_) = self.operation_manager.cancel_operation(&operation_id).await {
                    // Operation was cancelled successfully
                }

                Ok(())
            }
            _ => Err(format!("Cannot cancel request with status: {:?}", request.status).into())
        }
    }

    pub async fn get_analysis_status(
        &self,
        request_id: String,
    ) -> Result<RetrospectRequest, Box<dyn std::error::Error + Send + Sync>> {
        self.request_repo.find_by_id(&request_id).await?
            .ok_or("Request not found".into())
    }

    pub async fn get_analysis_result(
        &self,
        request_id: String,
    ) -> Result<Option<Retrospection>, Box<dyn std::error::Error + Send + Sync>> {
        let retrospections = self.retrospection_repo.find_by_request_id(&request_id).await?;

        // Return the most recent retrospection for this request
        Ok(retrospections.into_iter().next())
    }

    pub async fn list_analyses(
        &self,
        session_id: Option<String>,
        limit: Option<usize>,
    ) -> Result<Vec<RetrospectRequest>, Box<dyn std::error::Error + Send + Sync>> {
        match session_id {
            Some(session_id) => {
                self.request_repo.find_by_session_id(&session_id).await
            }
            None => {
                self.request_repo.find_recent(limit).await
            }
        }
    }

    pub async fn get_active_analyses(
        &self,
    ) -> Result<Vec<RetrospectRequest>, Box<dyn std::error::Error + Send + Sync>> {
        self.request_repo.find_active_requests().await
    }

    pub async fn cleanup_old_analyses(
        &self,
        days_old: u32,
    ) -> Result<u64, Box<dyn std::error::Error + Send + Sync>> {
        let cutoff_date = chrono::Utc::now() - chrono::Duration::days(days_old as i64);

        // Clean up old retrospections first (due to foreign key constraints)
        let retrospections_deleted = self.retrospection_repo.delete_before(cutoff_date).await?;

        // Then clean up old completed requests
        let requests_deleted = self.request_repo.delete_completed_before(cutoff_date).await?;

        Ok(retrospections_deleted + requests_deleted)
    }

    async fn perform_analysis(
        &self,
        request: &RetrospectRequest,
    ) -> Result<Retrospection, Box<dyn std::error::Error + Send + Sync>> {
        // In a real implementation, this would:
        // 1. Fetch chat session data
        // 2. Prepare the analysis prompt based on the analysis type
        // 3. Call Google AI API with the data and prompt
        // 4. Process the response and extract insights

        // For now, we'll create a mock analysis based on the type
        let (insights, reflection, recommendations) = self.generate_mock_analysis(&request.analysis_type)?;

        let retrospection = Retrospection::new(
            request.id.clone(),
            insights,
            reflection,
            recommendations,
            None, // metadata
        );

        Ok(retrospection)
    }

    fn generate_mock_analysis(
        &self,
        analysis_type: &RetrospectionAnalysisType,
    ) -> Result<(String, String, String), Box<dyn std::error::Error + Send + Sync>> {
        let (insights, reflection, recommendations) = match analysis_type {
            RetrospectionAnalysisType::UserInteractionAnalysis => (
                "User interaction patterns show consistent engagement with technical questions and requests for detailed explanations.".to_string(),
                "The user demonstrates a preference for thorough, step-by-step guidance and appreciates when examples are provided.".to_string(),
                "Continue providing detailed explanations with examples. Consider offering alternative approaches to problems.".to_string(),
            ),
            RetrospectionAnalysisType::CollaborationInsights => (
                "Collaboration patterns indicate effective back-and-forth communication with clear problem identification.".to_string(),
                "The working relationship shows good technical communication and mutual understanding of goals.".to_string(),
                "Maintain current communication style. Consider suggesting more proactive approaches to anticipating needs.".to_string(),
            ),
            RetrospectionAnalysisType::QuestionQuality => (
                "Questions are generally well-structured and provide sufficient context for meaningful responses.".to_string(),
                "There's a good balance between specific technical questions and broader conceptual inquiries.".to_string(),
                "Encourage continued specificity in technical questions. Consider asking for examples when concepts are unclear.".to_string(),
            ),
            RetrospectionAnalysisType::TaskBreakdown => (
                "Task decomposition shows logical progression from high-level goals to specific implementation steps.".to_string(),
                "Complex problems are being broken down effectively into manageable components.".to_string(),
                "Continue breaking down complex tasks. Consider documenting decision points and rationale.".to_string(),
            ),
            RetrospectionAnalysisType::FollowUpPatterns => (
                "Several topics presented opportunities for deeper exploration or follow-up questions.".to_string(),
                "Some conversations ended at a surface level when deeper investigation might have been beneficial.".to_string(),
                "Consider asking follow-up questions about implementation details or alternative approaches.".to_string(),
            ),
            RetrospectionAnalysisType::Custom(_) => (
                "Custom analysis completed based on provided criteria.".to_string(),
                "Analysis tailored to specific requirements and context.".to_string(),
                "Review results against custom criteria and adjust approach as needed.".to_string(),
            ),
        };

        Ok((insights, reflection, recommendations))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::Database;

    #[tokio::test]
    async fn test_create_analysis_request() {
        let database = Database::new_in_memory().await.unwrap();
        database.initialize().await.unwrap();

        let service = RetrospectionService::new(
            Arc::new(database.manager),
            GoogleAiClient::new(GoogleAiConfig::default()).unwrap(),
        );

        let request = service.create_analysis_request(
            "session-123".to_string(),
            RetrospectionAnalysisType::UserInteractionAnalysis,
            Some("test_user".to_string()),
            None,
        ).await.unwrap();

        assert_eq!(request.session_id, "session-123");
        assert_eq!(request.analysis_type, RetrospectionAnalysisType::UserInteractionAnalysis);
        assert_eq!(request.status, OperationStatus::Pending);
    }

    #[tokio::test]
    async fn test_get_analysis_status() {
        let database = Database::new_in_memory().await.unwrap();
        database.initialize().await.unwrap();

        let service = RetrospectionService::new(
            Arc::new(database.manager),
            GoogleAiClient::new(GoogleAiConfig::default()).unwrap(),
        );

        let request = service.create_analysis_request(
            "session-456".to_string(),
            RetrospectionAnalysisType::CollaborationInsights,
            Some("test_user".to_string()),
            None,
        ).await.unwrap();

        let status = service.get_analysis_status(request.id.clone()).await.unwrap();
        assert_eq!(status.id, request.id);
        assert_eq!(status.status, OperationStatus::Pending);
    }
}