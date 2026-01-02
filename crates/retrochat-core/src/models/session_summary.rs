use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Outcome classification for a session
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub enum SessionOutcome {
    /// Session completed successfully
    Completed,
    /// Session partially completed
    Partial,
    /// Session was abandoned
    Abandoned,
    /// Session is ongoing (no clear end)
    #[default]
    Ongoing,
}

impl std::fmt::Display for SessionOutcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SessionOutcome::Completed => write!(f, "completed"),
            SessionOutcome::Partial => write!(f, "partial"),
            SessionOutcome::Abandoned => write!(f, "abandoned"),
            SessionOutcome::Ongoing => write!(f, "ongoing"),
        }
    }
}

impl std::str::FromStr for SessionOutcome {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "completed" => Ok(SessionOutcome::Completed),
            "partial" => Ok(SessionOutcome::Partial),
            "abandoned" => Ok(SessionOutcome::Abandoned),
            "ongoing" => Ok(SessionOutcome::Ongoing),
            _ => Err(format!("Unknown session outcome: {s}")),
        }
    }
}

/// LLM-generated session-level summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSummary {
    pub id: String,
    pub session_id: String,

    // LLM-generated content
    pub title: String,
    pub summary: String,
    pub primary_goal: Option<String>,
    pub outcome: Option<SessionOutcome>,

    // Extracted entities (JSON arrays stored as vectors)
    pub key_decisions: Option<Vec<String>>,
    pub technologies_used: Option<Vec<String>>,
    pub files_affected: Option<Vec<String>>,

    // Generation metadata
    pub model_used: Option<String>,
    pub prompt_version: i32,
    pub generated_at: DateTime<Utc>,
}

impl SessionSummary {
    /// Create a new SessionSummary with required fields
    pub fn new(session_id: String, title: String, summary: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            session_id,
            title,
            summary,
            primary_goal: None,
            outcome: None,
            key_decisions: None,
            technologies_used: None,
            files_affected: None,
            model_used: None,
            prompt_version: 1,
            generated_at: Utc::now(),
        }
    }

    pub fn with_primary_goal(mut self, goal: String) -> Self {
        self.primary_goal = Some(goal);
        self
    }

    pub fn with_outcome(mut self, outcome: SessionOutcome) -> Self {
        self.outcome = Some(outcome);
        self
    }

    pub fn with_key_decisions(mut self, decisions: Vec<String>) -> Self {
        self.key_decisions = Some(decisions);
        self
    }

    pub fn with_technologies_used(mut self, technologies: Vec<String>) -> Self {
        self.technologies_used = Some(technologies);
        self
    }

    pub fn with_files_affected(mut self, files: Vec<String>) -> Self {
        self.files_affected = Some(files);
        self
    }

    pub fn with_model_used(mut self, model: String) -> Self {
        self.model_used = Some(model);
        self
    }

    pub fn with_prompt_version(mut self, version: i32) -> Self {
        self.prompt_version = version;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_outcome_display() {
        assert_eq!(SessionOutcome::Completed.to_string(), "completed");
        assert_eq!(SessionOutcome::Partial.to_string(), "partial");
        assert_eq!(SessionOutcome::Abandoned.to_string(), "abandoned");
        assert_eq!(SessionOutcome::Ongoing.to_string(), "ongoing");
    }

    #[test]
    fn test_session_outcome_from_str() {
        assert_eq!(
            "completed".parse::<SessionOutcome>().unwrap(),
            SessionOutcome::Completed
        );
        assert_eq!(
            "partial".parse::<SessionOutcome>().unwrap(),
            SessionOutcome::Partial
        );
        assert!("invalid".parse::<SessionOutcome>().is_err());
    }

    #[test]
    fn test_new_session_summary() {
        let session_id = Uuid::new_v4().to_string();

        let summary = SessionSummary::new(
            session_id.clone(),
            "JWT Authentication Implementation".to_string(),
            "This session implemented JWT authentication for the API.".to_string(),
        );

        assert_eq!(summary.session_id, session_id);
        assert_eq!(summary.title, "JWT Authentication Implementation");
        assert!(summary.primary_goal.is_none());
        assert!(summary.outcome.is_none());
    }

    #[test]
    fn test_session_summary_builder_pattern() {
        let summary = SessionSummary::new(
            "session-1".to_string(),
            "Title".to_string(),
            "Summary".to_string(),
        )
        .with_primary_goal("Implement authentication".to_string())
        .with_outcome(SessionOutcome::Completed)
        .with_key_decisions(vec!["Used JWT".to_string(), "RS256 signing".to_string()])
        .with_technologies_used(vec!["JWT".to_string(), "bcrypt".to_string()])
        .with_files_affected(vec!["src/auth.rs".to_string()])
        .with_model_used("gemini-1.5-flash".to_string());

        assert_eq!(
            summary.primary_goal,
            Some("Implement authentication".to_string())
        );
        assert_eq!(summary.outcome, Some(SessionOutcome::Completed));
        assert_eq!(
            summary.key_decisions,
            Some(vec!["Used JWT".to_string(), "RS256 signing".to_string()])
        );
        assert_eq!(
            summary.technologies_used,
            Some(vec!["JWT".to_string(), "bcrypt".to_string()])
        );
        assert_eq!(
            summary.files_affected,
            Some(vec!["src/auth.rs".to_string()])
        );
        assert_eq!(summary.model_used, Some("gemini-1.5-flash".to_string()));
    }
}
