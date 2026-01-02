use anyhow::{Context, Result as AnyhowResult};
use regex::Regex;
use uuid::Uuid;

use crate::database::{DatabaseManager, SessionSummaryRepository, TurnSummaryRepository};
use crate::models::session_summary::{SessionOutcome, SessionSummary};
use crate::models::TurnSummary;
use crate::services::google_ai::GoogleAiClient;

/// Service for generating LLM-based session summaries from turn summaries
pub struct SessionSummarizer {
    turn_summary_repo: TurnSummaryRepository,
    session_summary_repo: SessionSummaryRepository,
    ai_client: GoogleAiClient,
}

impl SessionSummarizer {
    pub fn new(db: &DatabaseManager, ai_client: GoogleAiClient) -> Self {
        Self {
            turn_summary_repo: TurnSummaryRepository::new(db),
            session_summary_repo: SessionSummaryRepository::new(db),
            ai_client,
        }
    }

    /// Summarize a session from its turn summaries
    ///
    /// Primary path: Generate from turn summaries (efficient, small input)
    /// Fallback path: Not implemented yet (would use raw messages)
    pub async fn summarize_session(&self, session_id: &Uuid) -> AnyhowResult<SessionSummary> {
        // Get turn summaries for the session
        let turn_summaries = self
            .turn_summary_repo
            .get_by_session(session_id)
            .await
            .context("Failed to fetch turn summaries")?;

        if turn_summaries.is_empty() {
            anyhow::bail!("No turn summaries found for session. Run turn summarization first.");
        }

        // Delete existing session summary if any
        self.session_summary_repo
            .delete_by_session(session_id)
            .await
            .context("Failed to delete existing session summary")?;

        // Generate summary from turn summaries
        let summary = self
            .generate_from_turns(session_id, &turn_summaries)
            .await?;

        // Save the summary
        self.session_summary_repo
            .create(&summary)
            .await
            .context("Failed to save session summary")?;

        Ok(summary)
    }

    /// Generate a session summary from turn summaries
    async fn generate_from_turns(
        &self,
        session_id: &Uuid,
        turns: &[TurnSummary],
    ) -> AnyhowResult<SessionSummary> {
        let prompt = self.build_session_prompt(turns);

        let analysis_request = crate::services::google_ai::models::AnalysisRequest {
            prompt,
            max_tokens: Some(1024),
            temperature: Some(0.3),
        };

        let response = self.ai_client.analytics(analysis_request).await?;
        let parsed = Self::parse_session_response(&response.text)?;

        let summary = SessionSummary::new(session_id.to_string(), parsed.title, parsed.summary)
            .with_primary_goal(parsed.primary_goal)
            .with_outcome(parsed.outcome)
            .with_key_decisions(parsed.key_decisions)
            .with_technologies_used(parsed.technologies_used)
            .with_files_affected(parsed.files_affected)
            .with_model_used("gemini-1.5-flash".to_string());

        Ok(summary)
    }

    /// Build a prompt for session summarization from turn summaries
    fn build_session_prompt(&self, turns: &[TurnSummary]) -> String {
        let mut turns_text = String::new();

        for turn in turns {
            let turn_type = turn
                .turn_type
                .as_ref()
                .map(|t| t.to_string())
                .unwrap_or_else(|| "unknown".to_string());

            let topics = turn
                .key_topics
                .as_ref()
                .map(|t| t.join(", "))
                .unwrap_or_default();

            turns_text.push_str(&format!(
                "Turn {num} ({turn_type}): {summary}\n  Topics: {topics}\n\n",
                num = turn.turn_number + 1,
                summary = turn.summary,
            ));
        }

        format!(
            r#"Analyze the following session summary (derived from individual turn summaries) and provide a comprehensive session overview.

## Session Turns

{turns_text}

## Task

Create a high-level summary of this entire coding session by synthesizing the turn summaries above.

## Required Output Format

Your response MUST follow this exact format:

TITLE: [A concise title for the session, max 60 characters, e.g., "JWT Authentication Implementation"]

SUMMARY: [A 2-3 sentence overview of what was accomplished in the session]

PRIMARY_GOAL: [The main objective the user was trying to achieve]

OUTCOME: [One of: completed, partial, abandoned, ongoing]

KEY_DECISIONS: [Comma-separated list of important decisions made]

TECHNOLOGIES_USED: [Comma-separated list of technologies, frameworks, or tools used]

FILES_AFFECTED: [Comma-separated list of key files that were created or modified]

Example:

TITLE: JWT Authentication Implementation

SUMMARY: Implemented complete JWT-based authentication system with middleware, token validation, and refresh token support. Added comprehensive tests and updated API documentation.

PRIMARY_GOAL: Add secure authentication to the REST API

OUTCOME: completed

KEY_DECISIONS: Used RS256 over HS256 for token signing, Added refresh tokens for better UX

TECHNOLOGIES_USED: JWT, bcrypt, axum, tokio

FILES_AFFECTED: src/auth/mod.rs, src/middleware/auth.rs, tests/auth_tests.rs"#,
            turns_text = turns_text.trim()
        )
    }

    /// Parse the LLM response for session summarization
    fn parse_session_response(response: &str) -> AnyhowResult<ParsedSessionResponse> {
        let title = Self::extract_field(response, "TITLE")
            .unwrap_or_else(|| "Untitled Session".to_string());

        let summary = Self::extract_field(response, "SUMMARY")
            .unwrap_or_else(|| "No summary available".to_string());

        let primary_goal = Self::extract_field(response, "PRIMARY_GOAL")
            .unwrap_or_else(|| "Unknown goal".to_string());

        let outcome_str = Self::extract_field(response, "OUTCOME").unwrap_or_default();
        let outcome = outcome_str
            .to_lowercase()
            .parse::<SessionOutcome>()
            .unwrap_or(SessionOutcome::Ongoing);

        let key_decisions =
            Self::parse_list(&Self::extract_field(response, "KEY_DECISIONS").unwrap_or_default());

        let technologies_used = Self::parse_list(
            &Self::extract_field(response, "TECHNOLOGIES_USED").unwrap_or_default(),
        );

        let files_affected =
            Self::parse_list(&Self::extract_field(response, "FILES_AFFECTED").unwrap_or_default());

        Ok(ParsedSessionResponse {
            title,
            summary,
            primary_goal,
            outcome,
            key_decisions,
            technologies_used,
            files_affected,
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

    /// Parse a comma-separated list into a Vec<String>
    fn parse_list(input: &str) -> Vec<String> {
        input
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()
    }

    /// Check if a session has been summarized
    pub async fn is_session_summarized(&self, session_id: &Uuid) -> AnyhowResult<bool> {
        self.session_summary_repo
            .exists_for_session(session_id)
            .await
    }

    /// Get existing session summary
    pub async fn get_session_summary(
        &self,
        session_id: &Uuid,
    ) -> AnyhowResult<Option<SessionSummary>> {
        self.session_summary_repo.get_by_session(session_id).await
    }
}

/// Parsed response from session summarization LLM call
struct ParsedSessionResponse {
    title: String,
    summary: String,
    primary_goal: String,
    outcome: SessionOutcome,
    key_decisions: Vec<String>,
    technologies_used: Vec<String>,
    files_affected: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_field() {
        let response = r#"
TITLE: JWT Authentication
SUMMARY: Implemented JWT auth
OUTCOME: completed
"#;

        assert_eq!(
            SessionSummarizer::extract_field(response, "TITLE"),
            Some("JWT Authentication".to_string())
        );
        assert_eq!(
            SessionSummarizer::extract_field(response, "OUTCOME"),
            Some("completed".to_string())
        );
    }

    #[test]
    fn test_parse_list() {
        let input = "JWT, bcrypt, axum, tokio";
        let result = SessionSummarizer::parse_list(input);
        assert_eq!(
            result,
            vec![
                "JWT".to_string(),
                "bcrypt".to_string(),
                "axum".to_string(),
                "tokio".to_string()
            ]
        );
    }

    #[test]
    fn test_parse_list_empty() {
        let input = "";
        let result = SessionSummarizer::parse_list(input);
        assert!(result.is_empty());
    }

    #[test]
    fn test_parse_session_response() {
        let response = r#"
TITLE: User Authentication System

SUMMARY: Built a complete authentication system with login, registration, and password reset functionality.

PRIMARY_GOAL: Implement user authentication

OUTCOME: completed

KEY_DECISIONS: Used JWT for sessions, Added email verification

TECHNOLOGIES_USED: JWT, bcrypt, sendgrid

FILES_AFFECTED: src/auth.rs, src/routes/auth.rs
"#;

        let parsed = SessionSummarizer::parse_session_response(response).unwrap();

        assert_eq!(parsed.title, "User Authentication System");
        assert!(parsed.summary.contains("authentication"));
        assert_eq!(parsed.primary_goal, "Implement user authentication");
        assert_eq!(parsed.outcome, SessionOutcome::Completed);
        assert_eq!(parsed.key_decisions.len(), 2);
        assert_eq!(parsed.technologies_used.len(), 3);
        assert_eq!(parsed.files_affected.len(), 2);
    }

    #[test]
    fn test_parse_session_response_missing_fields() {
        let response = "Some malformed response";

        let parsed = SessionSummarizer::parse_session_response(response).unwrap();

        assert_eq!(parsed.title, "Untitled Session");
        assert_eq!(parsed.summary, "No summary available");
        assert_eq!(parsed.outcome, SessionOutcome::Ongoing);
    }
}
