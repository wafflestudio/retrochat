use std::sync::Arc;

use crate::database::{DatabaseManager, FlowchartRepository, MessageRepository};
use crate::models::{Flowchart, Message};
use crate::services::google_ai::{GenerateContentRequest, GoogleAiClient};

pub struct FlowchartService {
    google_ai_client: GoogleAiClient,
    flowchart_repo: FlowchartRepository,
    message_repo: MessageRepository,
}

impl FlowchartService {
    pub fn new(db_manager: Arc<DatabaseManager>, google_ai_client: GoogleAiClient) -> Self {
        let flowchart_repo = FlowchartRepository::new(db_manager.clone());
        let message_repo = MessageRepository::new(&db_manager);

        Self {
            google_ai_client,
            flowchart_repo,
            message_repo,
        }
    }

    /// Generate or retrieve a flowchart for a session
    /// Returns cached flowchart if exists, otherwise generates a new one
    pub async fn get_or_generate_flowchart(
        &self,
        session_id: &str,
    ) -> Result<Flowchart, Box<dyn std::error::Error + Send + Sync>> {
        // Check if flowchart already exists
        let existing = self.flowchart_repo.get_by_session_id(session_id).await?;
        if let Some(flowchart) = existing.first() {
            return Ok(flowchart.clone());
        }

        // Generate new flowchart
        self.generate_flowchart(session_id).await
    }

    /// Force generate a new flowchart for a session
    pub async fn generate_flowchart(
        &self,
        session_id: &str,
    ) -> Result<Flowchart, Box<dyn std::error::Error + Send + Sync>> {
        // Get all messages for this session
        let session_uuid = uuid::Uuid::parse_str(session_id)?;
        let messages = self.message_repo.get_by_session_id(&session_uuid).await?;

        if messages.is_empty() {
            return Err("No messages found for this session".into());
        }

        // Build prompt for Google AI
        let prompt = self.build_flowchart_prompt(&messages);

        // Call Google AI
        let request = GenerateContentRequest::new(prompt);
        let response = self.google_ai_client.generate_content(request).await?;

        // Parse response
        let response_text = response
            .extract_text()
            .ok_or("Failed to extract text from AI response")?;

        // Parse JSON response
        let flowchart_data: FlowchartResponse = serde_json::from_str(&response_text)
            .or_else(|_| self.extract_json_from_markdown(&response_text))?;

        // Create Flowchart model
        let mut flowchart = Flowchart::new(
            session_id.to_string(),
            flowchart_data.nodes,
            flowchart_data.edges,
        );

        if let Some(token_usage) = response.get_token_usage() {
            flowchart = flowchart.with_token_usage(token_usage);
        }

        // Validate DAG
        if !flowchart.is_valid_dag() {
            return Err("Generated flowchart contains cycles (not a valid DAG)".into());
        }

        // Save to database
        self.flowchart_repo.create(&flowchart).await?;

        Ok(flowchart)
    }

    fn build_flowchart_prompt(&self, messages: &[Message]) -> String {
        let mut conversation = String::new();

        for (idx, msg) in messages.iter().enumerate() {
            // Truncate long messages to save tokens
            let content = if msg.content.len() > 200 {
                format!("{}...", &msg.content[..200])
            } else {
                msg.content.clone()
            };
            conversation.push_str(&format!(
                "\n[Message {}] Role: {:?}\nContent: {}\n",
                idx + 1,
                msg.role,
                content
            ));
        }

        format!(
            r#"Create a simple flowchart with MAXIMUM 8 NODES. Each node represents a major work phase.

CONVERSATION:
{}

Return ONLY this JSON (no markdown, no extra text):
{{
  "nodes": [
    {{"id": "1", "label": "Phase Name", "message_refs": [{{"message_id": "1", "sequence_number": 1, "portion": null}}], "node_type": "action", "description": null}}
  ],
  "edges": [
    {{"from_node": "1", "to_node": "2", "edge_type": "sequential", "label": null}}
  ]
}}

RULES:
- MAX 8 nodes only
- Each message_refs array: MAX 3 items only
- Always "portion": null
- node_type: "action", "context", "decision", "start", "end"
- edge_type: "sequential", "merge", "branch"
"#,
            conversation
        )
    }

    /// Extract JSON from markdown code blocks if the AI wrapped it
    fn extract_json_from_markdown(
        &self,
        text: &str,
    ) -> Result<FlowchartResponse, Box<dyn std::error::Error + Send + Sync>> {
        // Try to find JSON in markdown code blocks
        if let Some(start) = text.find("```json") {
            let search_start = start + 7; // Start after "```json"
            if let Some(end_offset) = text[search_start..].find("```") {
                let end = search_start + end_offset;
                let json_str = &text[search_start..end].trim();
                return Ok(serde_json::from_str(json_str)?);
            }
        }

        // Try to find JSON without markdown wrapper
        if let Some(start) = text.find('{') {
            if let Some(end) = text.rfind('}') {
                let json_str = &text[start..=end].trim();
                return Ok(serde_json::from_str(json_str)?);
            }
        }

        Err("Could not extract valid JSON from AI response".into())
    }

    /// Delete flowchart for a session (to force regeneration)
    pub async fn delete_flowchart(
        &self,
        session_id: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.flowchart_repo.delete_by_session_id(session_id).await?;
        Ok(())
    }
}

/// Temporary structure for parsing AI response
#[derive(serde::Deserialize)]
struct FlowchartResponse {
    nodes: Vec<crate::models::FlowchartNode>,
    edges: Vec<crate::models::FlowchartEdge>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::Database;
    use crate::models::{MessageRole, Project};
    use crate::services::google_ai::GoogleAiConfig;

    #[tokio::test]
    #[ignore] // Requires GOOGLE_AI_API_KEY
    async fn test_generate_flowchart() {
        let db = Database::new_in_memory().await.unwrap();
        db.initialize().await.unwrap();

        // Create test project and session
        let project_repo = db.project_repo();
        let project = Project::new("test-project".to_string());
        project_repo.create(&project).await.unwrap();

        let session_repo = db.chat_session_repo();
        let session = crate::models::ChatSession::new(
            crate::models::Provider::ClaudeCode,
            "test-provider".to_string(),
            "test-hash".to_string(),
            chrono::Utc::now(),
        );
        session_repo.create(&session).await.unwrap();

        // Create test messages
        let message_repo = db.message_repo();
        let msg1 = Message::new(
            session.id.clone(),
            MessageRole::User,
            "Create a todo list application".to_string(),
            chrono::Utc::now(),
            1,
        );
        let msg2 = Message::new(
            session.id.clone(),
            MessageRole::Assistant,
            "I'll create a todo list app for you.".to_string(),
            chrono::Utc::now(),
            2,
        );
        message_repo.create(&msg1).await.unwrap();
        message_repo.create(&msg2).await.unwrap();

        // Create service
        let api_key = std::env::var("GOOGLE_AI_API_KEY").unwrap();
        let config = GoogleAiConfig::new(api_key);
        let client = GoogleAiClient::new(config).unwrap();
        let service = FlowchartService::new(Arc::new(db.manager), client);

        // Generate flowchart
        let flowchart = service
            .generate_flowchart(&session.id.to_string())
            .await
            .unwrap();

        assert!(!flowchart.nodes.is_empty());
        assert!(flowchart.is_valid_dag());
    }
}
