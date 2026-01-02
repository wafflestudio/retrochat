use anyhow::{Context, Result as AnyhowResult};
use regex::Regex;
use std::sync::Arc;
use uuid::Uuid;

use crate::database::{DatabaseManager, MessageRepository, TurnSummaryRepository};
use crate::models::message::MessageType;
use crate::models::{DetectedTurn, Message, MessageRole, TurnSummary, TurnType};
use crate::services::llm::{GenerateRequest, LlmClient};
use crate::services::turn_detection::TurnDetector;

/// Service for generating LLM-based turn summaries
pub struct TurnSummarizer {
    message_repo: MessageRepository,
    turn_summary_repo: TurnSummaryRepository,
    turn_detector: TurnDetector,
    llm_client: Arc<dyn LlmClient>,
}

impl TurnSummarizer {
    pub fn new(db: &DatabaseManager, llm_client: Arc<dyn LlmClient>) -> Self {
        Self {
            message_repo: MessageRepository::new(db),
            turn_summary_repo: TurnSummaryRepository::new(db),
            turn_detector: TurnDetector::new(db),
            llm_client,
        }
    }

    /// Summarize all turns for a session
    ///
    /// Returns the number of turns summarized
    pub async fn summarize_session(&self, session_id: &Uuid) -> AnyhowResult<usize> {
        // Detect turns
        let turns = self
            .turn_detector
            .detect_turns(session_id)
            .await
            .context("Failed to detect turns")?;

        if turns.is_empty() {
            return Ok(0);
        }

        // Get all messages for the session
        let messages = self
            .message_repo
            .get_by_session(session_id)
            .await
            .context("Failed to fetch messages")?;

        // Delete existing summaries for this session
        self.turn_summary_repo
            .delete_by_session(session_id)
            .await
            .context("Failed to delete existing turn summaries")?;

        let mut summarized_count = 0;

        // Summarize each turn
        for turn in &turns {
            let turn_messages: Vec<&Message> = messages
                .iter()
                .filter(|m| {
                    m.sequence_number >= turn.start_sequence as u32
                        && m.sequence_number <= turn.end_sequence as u32
                })
                .collect();

            if turn_messages.is_empty() {
                continue;
            }

            match self.summarize_turn(session_id, turn, &turn_messages).await {
                Ok(summary) => {
                    self.turn_summary_repo
                        .create(&summary)
                        .await
                        .context("Failed to save turn summary")?;
                    summarized_count += 1;
                }
                Err(e) => {
                    tracing::warn!(
                        "Failed to summarize turn {} for session {}: {}",
                        turn.turn_number,
                        session_id,
                        e
                    );
                }
            }
        }

        Ok(summarized_count)
    }

    /// Summarize a single turn
    async fn summarize_turn(
        &self,
        session_id: &Uuid,
        turn: &DetectedTurn,
        messages: &[&Message],
    ) -> AnyhowResult<TurnSummary> {
        let prompt = self.build_turn_prompt(messages);

        let request = GenerateRequest::new(prompt)
            .with_max_tokens(1024)
            .with_temperature(0.3); // Lower temperature for more consistent output

        let response = self.llm_client.generate(request).await?;
        let parsed = Self::parse_turn_response(&response.text)?;

        let summary = TurnSummary::new(
            session_id.to_string(),
            turn.turn_number,
            turn.start_sequence,
            turn.end_sequence,
            parsed.user_intent,
            parsed.assistant_action,
            parsed.summary,
            turn.started_at,
            turn.ended_at,
        )
        .with_turn_type(parsed.turn_type)
        .with_key_topics(parsed.key_topics)
        .with_model_used(self.llm_client.model_name().to_string());

        Ok(summary)
    }

    /// Build a prompt for turn summarization
    fn build_turn_prompt(&self, messages: &[&Message]) -> String {
        let mut transcript = String::new();

        for msg in messages {
            let role = match msg.role {
                MessageRole::User => "USER",
                MessageRole::Assistant => "ASSISTANT",
                MessageRole::System => "SYSTEM",
            };

            let msg_type = match msg.message_type {
                MessageType::ToolRequest => " [Tool Request]",
                MessageType::ToolResult => " [Tool Result]",
                MessageType::Thinking => " [Thinking]",
                MessageType::SlashCommand => " [Command]",
                MessageType::SimpleMessage => "",
            };

            transcript.push_str(&format!(
                "[{role}{msg_type}]: {content}\n\n",
                content = Self::truncate_content(&msg.content, 1000)
            ));
        }

        format!(
            r#"Analyze the following turn from a coding assistant conversation and provide a structured summary.

## Turn Transcript

{transcript}

## Task

Summarize this turn by extracting:
1. What the user wanted to accomplish
2. What the assistant did in response
3. A brief combined summary
4. The type of turn (task, question, error_fix, clarification, or discussion)
5. Key topics discussed

## Required Output Format

Your response MUST follow this exact format:

USER_INTENT: [One sentence describing what the user wanted]

ASSISTANT_ACTION: [One sentence describing what the assistant did]

SUMMARY: [One sentence combining the above into a cohesive summary]

TURN_TYPE: [One of: task, question, error_fix, clarification, discussion]

KEY_TOPICS: [Comma-separated list of 2-5 key topics/technologies mentioned]

Example:

USER_INTENT: User wanted to add JWT authentication to the API endpoints.

ASSISTANT_ACTION: Created auth middleware and JWT validation logic.

SUMMARY: Implemented JWT authentication with middleware for protecting API routes.

TURN_TYPE: task

KEY_TOPICS: JWT, authentication, middleware, API security"#,
            transcript = transcript.trim()
        )
    }

    /// Truncate content to a maximum number of characters, preserving word boundaries
    /// Uses char_indices() to safely handle multi-byte UTF-8 characters
    fn truncate_content(content: &str, max_chars: usize) -> String {
        let char_count = content.chars().count();
        if char_count <= max_chars {
            return content.to_string();
        }

        // Find the byte index for the max_chars boundary
        let end_idx = content
            .char_indices()
            .nth(max_chars)
            .map(|(idx, _)| idx)
            .unwrap_or(content.len());

        let truncated = &content[..end_idx];

        // Try to break at a word boundary
        if let Some(last_space) = truncated.rfind(char::is_whitespace) {
            format!("{}...", &truncated[..last_space])
        } else {
            format!("{}...", truncated)
        }
    }

    /// Parse the LLM response for turn summarization
    fn parse_turn_response(response: &str) -> AnyhowResult<ParsedTurnResponse> {
        let user_intent = Self::extract_field(response, "USER_INTENT")
            .unwrap_or_else(|| "Unknown intent".to_string());

        let assistant_action = Self::extract_field(response, "ASSISTANT_ACTION")
            .unwrap_or_else(|| "Unknown action".to_string());

        let summary = Self::extract_field(response, "SUMMARY")
            .unwrap_or_else(|| format!("{} -> {}", user_intent, assistant_action));

        let turn_type_str = Self::extract_field(response, "TURN_TYPE").unwrap_or_default();
        let turn_type = turn_type_str
            .to_lowercase()
            .parse::<TurnType>()
            .unwrap_or(TurnType::Discussion);

        let key_topics_str = Self::extract_field(response, "KEY_TOPICS").unwrap_or_default();
        let key_topics: Vec<String> = key_topics_str
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        Ok(ParsedTurnResponse {
            user_intent,
            assistant_action,
            summary,
            turn_type,
            key_topics,
        })
    }

    /// Extract a field from the response
    fn extract_field(response: &str, field_name: &str) -> Option<String> {
        let pattern = format!(r"(?i){}:\s*(.+)", regex::escape(field_name));
        let re = Regex::new(&pattern).ok()?;

        re.captures(response)
            .and_then(|caps| caps.get(1))
            .map(|m| m.as_str().trim().to_string())
    }

    /// Check if a session has been summarized
    pub async fn is_session_summarized(&self, session_id: &Uuid) -> AnyhowResult<bool> {
        let count = self.turn_summary_repo.count_by_session(session_id).await?;
        Ok(count > 0)
    }

    /// Get existing turn summaries for a session
    pub async fn get_session_turns(&self, session_id: &Uuid) -> AnyhowResult<Vec<TurnSummary>> {
        self.turn_summary_repo.get_by_session(session_id).await
    }
}

/// Parsed response from turn summarization LLM call
struct ParsedTurnResponse {
    user_intent: String,
    assistant_action: String,
    summary: String,
    turn_type: TurnType,
    key_topics: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate_content_short() {
        let content = "Short content";
        assert_eq!(TurnSummarizer::truncate_content(content, 100), content);
    }

    #[test]
    fn test_truncate_content_long() {
        let content = "This is a very long piece of content that needs to be truncated";
        let truncated = TurnSummarizer::truncate_content(content, 20);
        assert!(truncated.chars().count() <= 23); // 20 chars + "..."
        assert!(truncated.ends_with("..."));
    }

    #[test]
    fn test_truncate_content_unicode_emoji() {
        // Each emoji is one character but multiple bytes
        let content = "Hello 🎉🎊🎁🎈🎂 World";
        // Truncate at 10 chars: "Hello 🎉🎊🎁🎈" (10 chars)
        let truncated = TurnSummarizer::truncate_content(content, 10);
        assert!(truncated.ends_with("..."));
        // Should not panic and should handle multi-byte chars correctly
        assert!(truncated.chars().count() <= 13); // 10 + "..."
    }

    #[test]
    fn test_truncate_content_unicode_cjk() {
        // CJK characters are multi-byte
        let content = "안녕하세요 세계입니다";
        // Truncate at 5 chars
        let truncated = TurnSummarizer::truncate_content(content, 5);
        assert!(truncated.ends_with("..."));
        // "안녕하세요" is 5 chars, but there's a space so it may break at "안녕하세요"
        assert!(truncated.chars().count() <= 8); // 5 + "..."
    }

    #[test]
    fn test_truncate_content_unicode_mixed() {
        // Mix of ASCII, emoji, and CJK
        let content = "Hello世界🌍Test";
        let truncated = TurnSummarizer::truncate_content(content, 8);
        assert!(truncated.ends_with("..."));
        // Should handle mixed content without panic
    }

    #[test]
    fn test_extract_field() {
        let response = r#"
USER_INTENT: Add authentication
ASSISTANT_ACTION: Created JWT module
SUMMARY: Implemented JWT auth
TURN_TYPE: task
KEY_TOPICS: JWT, auth, middleware
"#;

        assert_eq!(
            TurnSummarizer::extract_field(response, "USER_INTENT"),
            Some("Add authentication".to_string())
        );
        assert_eq!(
            TurnSummarizer::extract_field(response, "TURN_TYPE"),
            Some("task".to_string())
        );
        assert_eq!(
            TurnSummarizer::extract_field(response, "KEY_TOPICS"),
            Some("JWT, auth, middleware".to_string())
        );
    }

    #[test]
    fn test_parse_turn_response() {
        let response = r#"
USER_INTENT: User wanted to add logging to the application
ASSISTANT_ACTION: Created a logging module with tracing support
SUMMARY: Implemented structured logging using tracing crate
TURN_TYPE: task
KEY_TOPICS: logging, tracing, observability
"#;

        let parsed = TurnSummarizer::parse_turn_response(response).unwrap();

        assert_eq!(
            parsed.user_intent,
            "User wanted to add logging to the application"
        );
        assert_eq!(
            parsed.assistant_action,
            "Created a logging module with tracing support"
        );
        assert_eq!(
            parsed.summary,
            "Implemented structured logging using tracing crate"
        );
        assert_eq!(parsed.turn_type, TurnType::Task);
        assert_eq!(
            parsed.key_topics,
            vec![
                "logging".to_string(),
                "tracing".to_string(),
                "observability".to_string()
            ]
        );
    }

    #[test]
    fn test_parse_turn_response_missing_fields() {
        let response = r#"
Some malformed response without proper fields
"#;

        let parsed = TurnSummarizer::parse_turn_response(response).unwrap();

        assert_eq!(parsed.user_intent, "Unknown intent");
        assert_eq!(parsed.assistant_action, "Unknown action");
        assert_eq!(parsed.turn_type, TurnType::Discussion);
        assert!(parsed.key_topics.is_empty());
    }
}
