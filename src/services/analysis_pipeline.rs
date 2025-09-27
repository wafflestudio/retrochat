use std::sync::Arc;

use crate::database::{DatabaseManager, ChatSessionRepository, MessageRepository};
use crate::models::{RetrospectRequest, RetrospectionAnalysisType, ChatSession, Message};
use crate::services::google_ai::{GoogleAiClient, AnalysisRequest, AnalysisResponse, GoogleAiConfig};

pub struct AnalysisPipeline {
    db_manager: Arc<DatabaseManager>,
    google_ai_client: GoogleAiClient,
    session_repo: ChatSessionRepository,
    message_repo: MessageRepository,
}

impl AnalysisPipeline {
    pub fn new(
        db_manager: Arc<DatabaseManager>,
        google_ai_client: GoogleAiClient,
    ) -> Self {
        let session_repo = ChatSessionRepository::new(&*db_manager);
        let message_repo = MessageRepository::new(&*db_manager);

        Self {
            db_manager,
            google_ai_client,
            session_repo,
            message_repo,
        }
    }

    pub async fn execute_analysis(
        &self,
        request: &RetrospectRequest,
    ) -> Result<AnalysisResponse, Box<dyn std::error::Error + Send + Sync>> {
        // Step 1: Gather data for analysis
        let analysis_data = self.gather_analysis_data(request).await?;

        // Step 2: Prepare the analysis prompt
        let prompt = self.prepare_analysis_prompt(request, &analysis_data)?;

        // Step 3: Execute the analysis via Google AI
        let analysis_request = AnalysisRequest {
            prompt,
            max_tokens: Some(4000),
            temperature: Some(0.7),
            model: Some("gemini-pro".to_string()),
        };

        let response = self.google_ai_client.analyze(analysis_request).await?;

        // Step 4: Post-process the results
        self.post_process_analysis(&response, request).await
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
                crate::models::MessageRole::User => "User",
                crate::models::MessageRole::Assistant => "Assistant",
                crate::models::MessageRole::System => "System",
            };
            let content_preview = if message.content.len() > 200 {
                format!("{}...", &message.content[..200])
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

    async fn post_process_analysis(
        &self,
        response: &AnalysisResponse,
        _request: &RetrospectRequest,
    ) -> Result<AnalysisResponse, Box<dyn std::error::Error + Send + Sync>> {
        // For now, just return the response as-is
        // In the future, this could include:
        // - Validation of the response format
        // - Extraction of structured data
        // - Quality scoring
        // - Metadata enrichment

        Ok(response.clone())
    }

    fn calculate_session_metrics(
        &self,
        _session: &ChatSession,
        messages: &[Message],
    ) -> Result<SessionMetrics, Box<dyn std::error::Error + Send + Sync>> {
        let message_count = messages.len();
        let user_messages: Vec<_> = messages.iter().filter(|m| m.role == crate::models::MessageRole::User).collect();
        let assistant_messages: Vec<_> = messages.iter().filter(|m| m.role == crate::models::MessageRole::Assistant).collect();

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::Database;

    #[tokio::test]
    async fn test_calculate_session_metrics() {
        let database = Database::new_in_memory().await.unwrap();
        database.initialize().await.unwrap();

        let pipeline = AnalysisPipeline::new(
            Arc::new(database.manager),
            GoogleAiClient::new(GoogleAiConfig::default()).unwrap(),
        );

        let messages = vec![
            Message {
                id: uuid::Uuid::new_v4(),
                session_id: uuid::Uuid::new_v4(),
                role: crate::models::MessageRole::User,
                content: "Hello there".to_string(),
                metadata: None,
                timestamp: chrono::Utc::now(),
                token_count: None,
                tool_calls: None,
                sequence_number: 0,
            },
            Message {
                id: uuid::Uuid::new_v4(),
                session_id: uuid::Uuid::new_v4(),
                role: crate::models::MessageRole::Assistant,
                content: "Hello! How can I help you?".to_string(),
                metadata: None,
                timestamp: chrono::Utc::now(),
                token_count: None,
                tool_calls: None,
                sequence_number: 0,
            },
        ];

        let session = ChatSession {
            id: uuid::Uuid::new_v4(),
            provider: crate::models::LlmProvider::ClaudeCode,
            project_name: None,
            start_time: chrono::Utc::now(),
            end_time: Some(chrono::Utc::now()),
            message_count: 0,
            token_count: None,
            file_path: "test".to_string(),
            file_hash: "test".to_string(),
            state: crate::models::SessionState::Created,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        let metrics = pipeline.calculate_session_metrics(&session, &messages).unwrap();

        assert_eq!(metrics.message_count, 2);
        assert_eq!(metrics.user_message_count, 1);
        assert_eq!(metrics.assistant_message_count, 1);
        assert_eq!(metrics.conversation_turns, 1);
    }

    #[tokio::test]
    async fn test_get_base_prompt() {
        let database = Database::new_in_memory().await.unwrap();
        database.initialize().await.unwrap();

        let pipeline = AnalysisPipeline::new(
            Arc::new(database.manager),
            GoogleAiClient::new(GoogleAiConfig::default()).unwrap(),
        );

        let prompt = pipeline.get_base_prompt(&RetrospectionAnalysisType::UserInteractionAnalysis);
        assert!(prompt.contains("user interaction patterns"));

        let prompt = pipeline.get_base_prompt(&RetrospectionAnalysisType::CollaborationInsights);
        assert!(prompt.contains("collaboration dynamics"));
    }
}