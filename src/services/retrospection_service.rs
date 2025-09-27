use std::sync::Arc;

use crate::database::{DatabaseManager, RetrospectRequestRepository, RetrospectionRepository, ChatSessionRepository, MessageRepository};
use crate::models::{RetrospectRequest, Retrospection, RetrospectionAnalysisType, OperationStatus, ChatSession, Message, MessageRole};
use crate::services::google_ai::{GoogleAiClient, AnalysisRequest, AnalysisResponse};

pub struct RetrospectionService {
    google_ai_client: GoogleAiClient,
    request_repo: RetrospectRequestRepository,
    retrospection_repo: RetrospectionRepository,
    session_repo: ChatSessionRepository,
    message_repo: MessageRepository,
}

impl RetrospectionService {
    pub fn new(
        db_manager: Arc<DatabaseManager>,
        google_ai_client: GoogleAiClient,
    ) -> Self {
        let request_repo = RetrospectRequestRepository::new(db_manager.clone());
        let retrospection_repo = RetrospectionRepository::new(db_manager.clone());
        let session_repo = ChatSessionRepository::new(&*db_manager);
        let message_repo = MessageRepository::new(&*db_manager);

        Self {
            google_ai_client,
            request_repo,
            retrospection_repo,
            session_repo,
            message_repo,
        }
    }

    pub async fn create_analysis_request(
        &self,
        session_id: String,
        analysis_type: RetrospectionAnalysisType,
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
            _ => {}
        }

        // Mark request as running
        request.mark_running();
        self.request_repo.update(&request).await?;

        // Perform the analysis synchronously (blocking for CLI, but TUI will handle async)
        match self.perform_analysis(&request).await {
            Ok(retrospection) => {
                // Save the retrospection result
                self.retrospection_repo.create(&retrospection).await?;

                // Mark request as completed
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
        let mut request = self.request_repo.find_by_id(&request_id).await?
            .ok_or("Request not found")?;

        // Only allow cancelling pending or running requests
        match request.status {
            OperationStatus::Pending | OperationStatus::Running => {
                request.mark_cancelled();
                self.request_repo.update(&request).await?;

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
        // Step 1: Gather data for analysis
        let analysis_data = self.gather_analysis_data(request).await?;

        // Step 2: Prepare the analysis prompt
        let prompt = self.prepare_analysis_prompt(request, &analysis_data)?;

        // Step 3: Execute the analysis via Google AI
        let analysis_request = AnalysisRequest {
            prompt,
            max_tokens: Some(4000),
            temperature: Some(0.7),
        };

        let response = self.google_ai_client.analyze(analysis_request).await?;

        // Step 4: Post-process the results and create retrospection
        let retrospection = self.create_retrospection_from_response(&response, request)?;

        Ok(retrospection)
    }

    async fn gather_analysis_data(
        &self,
        request: &RetrospectRequest,
    ) -> Result<AnalysisData, Box<dyn std::error::Error + Send + Sync>> {
        // Get the chat session
        let session = self.session_repo.get_by_id(&uuid::Uuid::parse_str(&request.session_id)?).await?
            .ok_or("Chat session not found")?;

        // Get messages for the session
        let messages = self.message_repo.get_by_session_id(&uuid::Uuid::parse_str(&request.session_id)?).await?;

        // Calculate session metrics
        let metrics = self.calculate_session_metrics(&session, &messages)?;

        Ok(AnalysisData {
            session,
            messages,
            metrics,
        })
    }

    fn prepare_analysis_prompt(
        &self,
        request: &RetrospectRequest,
        data: &AnalysisData,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let base_prompt = self.get_base_prompt(&request.analysis_type);
        let context = self.build_context(data)?;

        let prompt = if let Some(custom_prompt) = &request.custom_prompt {
            format!(
                "{}\n\nCustom Instructions: {}\n\nContext:\n{}\n\nPlease provide your analysis in JSON format with 'insights', 'reflection', and 'recommendations' fields.",
                base_prompt, custom_prompt, context
            )
        } else {
            format!(
                "{}\n\nContext:\n{}\n\nPlease provide your analysis in JSON format with 'insights', 'reflection', and 'recommendations' fields.",
                base_prompt, context
            )
        };

        Ok(prompt)
    }

    fn get_base_prompt(&self, analysis_type: &RetrospectionAnalysisType) -> &'static str {
        match analysis_type {
            RetrospectionAnalysisType::UserInteractionAnalysis => {
                "Analyze the user interaction patterns in this conversation. Focus on:\n\
                - Communication style and preferences\n\
                - Types of questions and requests\n\
                - Response patterns and engagement levels\n\
                - Areas where the user seemed most/least satisfied"
            }
            RetrospectionAnalysisType::CollaborationInsights => {
                "Analyze the collaboration dynamics in this conversation. Focus on:\n\
                - Quality of back-and-forth communication\n\
                - Problem-solving approaches\n\
                - Knowledge sharing and learning moments\n\
                - Opportunities for improved collaboration"
            }
            RetrospectionAnalysisType::QuestionQuality => {
                "Analyze the quality and effectiveness of questions in this conversation. Focus on:\n\
                - Clarity and specificity of questions\n\
                - Context provided with questions\n\
                - Question types and their appropriateness\n\
                - Opportunities for better question formulation"
            }
            RetrospectionAnalysisType::TaskBreakdown => {
                "Analyze how tasks and problems were broken down in this conversation. Focus on:\n\
                - Decomposition strategies used\n\
                - Logical flow from high-level to specific\n\
                - Identification of dependencies and blockers\n\
                - Effectiveness of the breakdown approach"
            }
            RetrospectionAnalysisType::FollowUpPatterns => {
                "Identify follow-up opportunities in this conversation. Focus on:\n\
                - Topics that could have been explored deeper\n\
                - Questions that weren't asked but should have been\n\
                - Implementation details that were skipped\n\
                - Areas for continued learning or exploration"
            }
            RetrospectionAnalysisType::Custom(_) => {
                "Perform a custom analysis of this conversation based on the provided instructions."
            }
        }
    }

    fn build_context(&self, data: &AnalysisData) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let mut context = String::new();

        // Session information
        context.push_str(&format!(
            "Session Information:\n\
            - ID: {}\n\
            - Project: {}\n\
            - Created: {}\n\
            - Duration: {} minutes\n\
            - Message Count: {}\n\
            - Provider: {}\n\n",
            data.session.id,
            data.session.project_name.as_deref().unwrap_or("Unknown"),
            data.session.created_at.format("%Y-%m-%d %H:%M:%S"),
            data.metrics.duration_minutes,
            data.metrics.message_count,
            data.session.provider
        ));

        // Conversation flow
        context.push_str("Conversation Flow:\n");
        for (i, message) in data.messages.iter().enumerate() {
            let role = match message.role {
                MessageRole::User => "User",
                MessageRole::Assistant => "Assistant",
                MessageRole::System => "System",
            };
            let content_preview = if message.content.len() > 200 {
                let truncated: String = message.content.chars().take(200).collect();
                format!("{}...", truncated)
            } else {
                message.content.clone()
            };

            context.push_str(&format!(
                "{}. {} ({}): {}\n",
                i + 1,
                role,
                message.timestamp.format("%H:%M:%S"),
                content_preview
            ));
        }

        // Session metrics
        context.push_str(&format!(
            "\nSession Metrics:\n\
            - Average message length: {} characters\n\
            - User messages: {}\n\
            - Assistant messages: {}\n\
            - Conversation turns: {}\n",
            data.metrics.avg_message_length,
            data.metrics.user_message_count,
            data.metrics.assistant_message_count,
            data.metrics.conversation_turns
        ));

        Ok(context)
    }

    fn create_retrospection_from_response(
        &self,
        response: &AnalysisResponse,
        request: &RetrospectRequest,
    ) -> Result<Retrospection, Box<dyn std::error::Error + Send + Sync>> {
        // Parse the JSON response to extract insights, reflection, and recommendations
        // For now, we'll use the response text directly and create a simple structure
        // In a real implementation, this would parse the JSON and extract structured data
        
        let insights = response.text.clone();
        let reflection = "Analysis completed successfully".to_string();
        let recommendations = "Review the insights above for actionable next steps".to_string();

        let retrospection = Retrospection::new(
            request.id.clone(),
            insights,
            reflection,
            recommendations,
            None, // metadata
        );

        Ok(retrospection)
    }

    fn calculate_session_metrics(
        &self,
        _session: &ChatSession,
        messages: &[Message],
    ) -> Result<SessionMetrics, Box<dyn std::error::Error + Send + Sync>> {
        let message_count = messages.len();
        let user_messages: Vec<_> = messages.iter().filter(|m| m.role == MessageRole::User).collect();
        let assistant_messages: Vec<_> = messages.iter().filter(|m| m.role == MessageRole::Assistant).collect();

        let total_chars: usize = messages.iter().map(|m| m.content.len()).sum();
        let avg_message_length = if message_count > 0 {
            total_chars / message_count
        } else {
            0
        };

        let duration_minutes = if let (Some(first), Some(last)) = (messages.first(), messages.last()) {
            let duration = last.timestamp - first.timestamp;
            duration.num_minutes() as u32
        } else {
            0
        };

        Ok(SessionMetrics {
            message_count: message_count as u32,
            user_message_count: user_messages.len() as u32,
            assistant_message_count: assistant_messages.len() as u32,
            avg_message_length: avg_message_length as u32,
            duration_minutes,
            conversation_turns: (message_count / 2) as u32, // Approximate
        })
    }
}

#[derive(Debug)]
pub struct AnalysisData {
    pub session: ChatSession,
    pub messages: Vec<Message>,
    pub metrics: SessionMetrics,
}

#[derive(Debug)]
pub struct SessionMetrics {
    pub message_count: u32,
    pub user_message_count: u32,
    pub assistant_message_count: u32,
    pub avg_message_length: u32,
    pub duration_minutes: u32,
    pub conversation_turns: u32,
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
        let _ = self.runtime.block_on(async move {
            match service.cancel_all_active_analyses().await {
                Ok(count) if count > 0 => {
                    eprintln!("Cancelled {} running retrospection requests due to CLI exit", count);
                }
                Ok(_) => {
                    // No active requests to cancel
                }
                Err(e) => {
                    eprintln!("Warning: Failed to cancel active retrospection requests: {}", e);
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
            crate::models::LlmProvider::ClaudeCode,
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
        let request = service.create_analysis_request(
            session_id.clone(),
            RetrospectionAnalysisType::UserInteractionAnalysis,
            Some("test_user".to_string()),
            None,
        ).await.unwrap();

        assert_eq!(request.session_id, session_id);
        assert_eq!(request.analysis_type, RetrospectionAnalysisType::UserInteractionAnalysis);
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
            crate::models::LlmProvider::ClaudeCode,
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
        let request = service.create_analysis_request(
            session_id,
            RetrospectionAnalysisType::CollaborationInsights,
            Some("test_user".to_string()),
            None,
        ).await.unwrap();

        let status = service.get_analysis_status(request.id.clone()).await.unwrap();
        assert_eq!(status.id, request.id);
        assert_eq!(status.status, OperationStatus::Pending);
    }
}