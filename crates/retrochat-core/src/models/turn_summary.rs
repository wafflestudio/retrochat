use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Classification of turn types based on user intent
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub enum TurnType {
    /// User wants to accomplish a specific task
    Task,
    /// User is asking a question
    Question,
    /// User is trying to fix an error
    ErrorFix,
    /// User is clarifying a previous request
    Clarification,
    /// General discussion or exploration
    #[default]
    Discussion,
}

impl std::fmt::Display for TurnType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TurnType::Task => write!(f, "task"),
            TurnType::Question => write!(f, "question"),
            TurnType::ErrorFix => write!(f, "error_fix"),
            TurnType::Clarification => write!(f, "clarification"),
            TurnType::Discussion => write!(f, "discussion"),
        }
    }
}

impl std::str::FromStr for TurnType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "task" => Ok(TurnType::Task),
            "question" => Ok(TurnType::Question),
            "error_fix" => Ok(TurnType::ErrorFix),
            "clarification" => Ok(TurnType::Clarification),
            "discussion" => Ok(TurnType::Discussion),
            _ => Err(format!("Unknown turn type: {s}")),
        }
    }
}

/// LLM-generated turn summary with message boundary references
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TurnSummary {
    pub id: String,
    pub session_id: String,
    pub turn_number: i32,

    // Message boundaries (references to messages table via sequence_number)
    pub start_sequence: i32,
    pub end_sequence: i32,

    // LLM-generated content
    pub user_intent: String,
    pub assistant_action: String,
    pub summary: String,

    // Classification
    pub turn_type: Option<TurnType>,

    // Extracted entities (JSON arrays stored as strings)
    pub key_topics: Option<Vec<String>>,
    pub decisions_made: Option<Vec<String>>,
    pub code_concepts: Option<Vec<String>>,

    // Cached timestamps (derived from messages)
    pub started_at: DateTime<Utc>,
    pub ended_at: DateTime<Utc>,

    // Generation metadata
    pub model_used: Option<String>,
    pub prompt_version: i32,
    pub generated_at: DateTime<Utc>,
}

impl TurnSummary {
    /// Create a new TurnSummary with required fields
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        session_id: String,
        turn_number: i32,
        start_sequence: i32,
        end_sequence: i32,
        user_intent: String,
        assistant_action: String,
        summary: String,
        started_at: DateTime<Utc>,
        ended_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            session_id,
            turn_number,
            start_sequence,
            end_sequence,
            user_intent,
            assistant_action,
            summary,
            turn_type: None,
            key_topics: None,
            decisions_made: None,
            code_concepts: None,
            started_at,
            ended_at,
            model_used: None,
            prompt_version: 1,
            generated_at: Utc::now(),
        }
    }

    pub fn with_turn_type(mut self, turn_type: TurnType) -> Self {
        self.turn_type = Some(turn_type);
        self
    }

    pub fn with_key_topics(mut self, topics: Vec<String>) -> Self {
        self.key_topics = Some(topics);
        self
    }

    pub fn with_decisions_made(mut self, decisions: Vec<String>) -> Self {
        self.decisions_made = Some(decisions);
        self
    }

    pub fn with_code_concepts(mut self, concepts: Vec<String>) -> Self {
        self.code_concepts = Some(concepts);
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

    /// Get the number of messages in this turn
    pub fn message_count(&self) -> i32 {
        self.end_sequence - self.start_sequence + 1
    }

    /// Convert turn summary to text for embedding generation.
    ///
    /// Combines relevant fields into a structured text format optimized
    /// for semantic embedding. The format includes labeled sections for
    /// better semantic understanding.
    pub fn to_embedding_text(&self) -> String {
        let mut parts = Vec::new();

        // Core LLM-generated content (always present)
        parts.push(format!("Intent: {}", self.user_intent));
        parts.push(format!("Action: {}", self.assistant_action));
        parts.push(format!("Summary: {}", self.summary));

        // Classification (helps with semantic clustering)
        if let Some(ref turn_type) = self.turn_type {
            parts.push(format!("Type: {}", turn_type));
        }

        // Extracted entities (searchable keywords)
        if let Some(ref topics) = self.key_topics {
            if !topics.is_empty() {
                parts.push(format!("Topics: {}", topics.join(", ")));
            }
        }
        if let Some(ref decisions) = self.decisions_made {
            if !decisions.is_empty() {
                parts.push(format!("Decisions: {}", decisions.join(", ")));
            }
        }
        if let Some(ref concepts) = self.code_concepts {
            if !concepts.is_empty() {
                parts.push(format!("Code concepts: {}", concepts.join(", ")));
            }
        }

        parts.join("\n\n")
    }

    /// Compute SHA256 hash of the embedding text for change detection.
    pub fn embedding_text_hash(&self) -> String {
        use sha2::{Digest, Sha256};
        let text = self.to_embedding_text();
        let mut hasher = Sha256::new();
        hasher.update(text.as_bytes());
        hex::encode(hasher.finalize())
    }
}

/// Detected turn boundaries before summarization
#[derive(Debug, Clone)]
pub struct DetectedTurn {
    pub turn_number: i32,
    pub start_sequence: i32,
    pub end_sequence: i32,
    pub started_at: DateTime<Utc>,
    pub ended_at: DateTime<Utc>,
}

impl DetectedTurn {
    pub fn new(
        turn_number: i32,
        start_sequence: i32,
        end_sequence: i32,
        started_at: DateTime<Utc>,
        ended_at: DateTime<Utc>,
    ) -> Self {
        Self {
            turn_number,
            start_sequence,
            end_sequence,
            started_at,
            ended_at,
        }
    }

    /// Get the number of messages in this turn
    pub fn message_count(&self) -> i32 {
        self.end_sequence - self.start_sequence + 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_turn_type_display() {
        assert_eq!(TurnType::Task.to_string(), "task");
        assert_eq!(TurnType::Question.to_string(), "question");
        assert_eq!(TurnType::ErrorFix.to_string(), "error_fix");
        assert_eq!(TurnType::Clarification.to_string(), "clarification");
        assert_eq!(TurnType::Discussion.to_string(), "discussion");
    }

    #[test]
    fn test_turn_type_from_str() {
        assert_eq!("task".parse::<TurnType>().unwrap(), TurnType::Task);
        assert_eq!("question".parse::<TurnType>().unwrap(), TurnType::Question);
        assert_eq!("error_fix".parse::<TurnType>().unwrap(), TurnType::ErrorFix);
        assert!("invalid".parse::<TurnType>().is_err());
    }

    #[test]
    fn test_new_turn_summary() {
        let session_id = Uuid::new_v4().to_string();
        let now = Utc::now();

        let summary = TurnSummary::new(
            session_id.clone(),
            0,
            1,
            5,
            "Add authentication".to_string(),
            "Created JWT auth module".to_string(),
            "User wanted auth, Claude created JWT module".to_string(),
            now,
            now,
        );

        assert_eq!(summary.session_id, session_id);
        assert_eq!(summary.turn_number, 0);
        assert_eq!(summary.start_sequence, 1);
        assert_eq!(summary.end_sequence, 5);
        assert_eq!(summary.message_count(), 5);
        assert!(summary.turn_type.is_none());
    }

    #[test]
    fn test_turn_summary_builder_pattern() {
        let now = Utc::now();
        let summary = TurnSummary::new(
            "session-1".to_string(),
            0,
            1,
            3,
            "intent".to_string(),
            "action".to_string(),
            "summary".to_string(),
            now,
            now,
        )
        .with_turn_type(TurnType::Task)
        .with_key_topics(vec!["auth".to_string(), "jwt".to_string()])
        .with_model_used("gemini-1.5-flash".to_string());

        assert_eq!(summary.turn_type, Some(TurnType::Task));
        assert_eq!(
            summary.key_topics,
            Some(vec!["auth".to_string(), "jwt".to_string()])
        );
        assert_eq!(summary.model_used, Some("gemini-1.5-flash".to_string()));
    }

    #[test]
    fn test_detected_turn() {
        let now = Utc::now();
        let turn = DetectedTurn::new(0, 1, 5, now, now);

        assert_eq!(turn.turn_number, 0);
        assert_eq!(turn.start_sequence, 1);
        assert_eq!(turn.end_sequence, 5);
        assert_eq!(turn.message_count(), 5);
    }

    #[test]
    fn test_to_embedding_text_minimal() {
        let now = Utc::now();
        let summary = TurnSummary::new(
            "session-1".to_string(),
            0,
            1,
            3,
            "Implement JWT auth".to_string(),
            "Created auth module".to_string(),
            "User wanted auth, created JWT module".to_string(),
            now,
            now,
        );

        let text = summary.to_embedding_text();
        assert!(text.contains("Intent: Implement JWT auth"));
        assert!(text.contains("Action: Created auth module"));
        assert!(text.contains("Summary: User wanted auth"));
        // Should not contain optional fields
        assert!(!text.contains("Type:"));
        assert!(!text.contains("Topics:"));
    }

    #[test]
    fn test_to_embedding_text_full() {
        let now = Utc::now();
        let summary = TurnSummary::new(
            "session-1".to_string(),
            0,
            1,
            3,
            "Implement JWT auth".to_string(),
            "Created auth module".to_string(),
            "User wanted auth, created JWT module".to_string(),
            now,
            now,
        )
        .with_turn_type(TurnType::Task)
        .with_key_topics(vec!["authentication".to_string(), "JWT".to_string()])
        .with_decisions_made(vec!["Use RS256".to_string()])
        .with_code_concepts(vec!["middleware".to_string()]);

        let text = summary.to_embedding_text();
        assert!(text.contains("Intent: Implement JWT auth"));
        assert!(text.contains("Type: task"));
        assert!(text.contains("Topics: authentication, JWT"));
        assert!(text.contains("Decisions: Use RS256"));
        assert!(text.contains("Code concepts: middleware"));
    }

    #[test]
    fn test_embedding_text_hash_consistency() {
        let now = Utc::now();
        let summary = TurnSummary::new(
            "session-1".to_string(),
            0,
            1,
            3,
            "intent".to_string(),
            "action".to_string(),
            "summary".to_string(),
            now,
            now,
        );

        let hash1 = summary.embedding_text_hash();
        let hash2 = summary.embedding_text_hash();
        assert_eq!(hash1, hash2);
        assert_eq!(hash1.len(), 64); // SHA256 hex = 64 chars
    }
}
