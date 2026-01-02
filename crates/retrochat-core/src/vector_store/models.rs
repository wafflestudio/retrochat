//! Data models for vector storage.

use chrono::{DateTime, Utc};

/// Embedding record for turn summaries stored in LanceDB.
#[derive(Debug, Clone)]
pub struct TurnEmbedding {
    /// Primary key (matches TurnSummary.id).
    pub id: String,

    /// Session ID for filtering.
    pub session_id: String,

    /// Turn number within session.
    pub turn_number: i32,

    /// Turn type classification (task, question, error_fix, etc.).
    pub turn_type: Option<String>,

    /// Turn start time for time-range queries.
    pub started_at: DateTime<Utc>,

    /// Turn end time.
    pub ended_at: DateTime<Utc>,

    /// The embedding vector (384 or 768 dimensions).
    pub embedding: Vec<f32>,

    /// SHA256 hash of the embedding text for change detection.
    pub text_hash: String,

    /// When the embedding was generated.
    pub embedded_at: DateTime<Utc>,

    /// Name of the model used to generate this embedding.
    pub model_name: String,
}

/// Embedding record for session summaries stored in LanceDB.
#[derive(Debug, Clone)]
pub struct SessionEmbedding {
    /// Primary key (matches SessionSummary.id).
    pub id: String,

    /// Session ID for joining with SQLite data.
    pub session_id: String,

    /// Session outcome classification.
    pub outcome: Option<String>,

    /// Session creation time.
    pub created_at: DateTime<Utc>,

    /// Last update time.
    pub updated_at: DateTime<Utc>,

    /// Provider name (claude, gemini, etc.).
    pub provider: String,

    /// Project name if available.
    pub project: Option<String>,

    /// The embedding vector (384 or 768 dimensions).
    pub embedding: Vec<f32>,

    /// SHA256 hash of the embedding text for change detection.
    pub text_hash: String,

    /// When the embedding was generated.
    pub embedded_at: DateTime<Utc>,

    /// Name of the model used to generate this embedding.
    pub model_name: String,
}

/// Result from a turn embedding search.
#[derive(Debug, Clone)]
pub struct TurnSearchResult {
    /// Turn embedding ID.
    pub id: String,

    /// Session ID.
    pub session_id: String,

    /// Turn number.
    pub turn_number: i32,

    /// Similarity score (0.0 to 1.0 for cosine similarity).
    pub score: f32,
}

/// Result from a session embedding search.
#[derive(Debug, Clone)]
pub struct SessionSearchResult {
    /// Session embedding ID.
    pub id: String,

    /// Session ID.
    pub session_id: String,

    /// Similarity score (0.0 to 1.0 for cosine similarity).
    pub score: f32,
}

/// Filter options for turn embedding searches.
#[derive(Debug, Clone, Default)]
pub struct TurnFilter {
    /// Filter by session ID.
    pub session_id: Option<String>,

    /// Filter by turn types.
    pub turn_types: Option<Vec<String>>,

    /// Filter by turns started after this time.
    pub started_after: Option<DateTime<Utc>>,

    /// Filter by turns started before this time.
    pub started_before: Option<DateTime<Utc>>,
}

impl TurnFilter {
    /// Create a new empty filter.
    pub fn new() -> Self {
        Self::default()
    }

    /// Filter by session ID.
    pub fn with_session_id(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }

    /// Filter by turn types.
    pub fn with_turn_types(mut self, types: Vec<String>) -> Self {
        self.turn_types = Some(types);
        self
    }

    /// Filter by time range.
    pub fn with_time_range(
        mut self,
        after: Option<DateTime<Utc>>,
        before: Option<DateTime<Utc>>,
    ) -> Self {
        self.started_after = after;
        self.started_before = before;
        self
    }

    /// Build a SQL WHERE clause for LanceDB.
    pub fn to_sql(&self) -> Option<String> {
        let mut conditions = Vec::new();

        if let Some(ref session_id) = self.session_id {
            conditions.push(format!("session_id = '{}'", session_id));
        }

        if let Some(ref types) = self.turn_types {
            if !types.is_empty() {
                let types_str = types
                    .iter()
                    .map(|t| format!("'{}'", t))
                    .collect::<Vec<_>>()
                    .join(", ");
                conditions.push(format!("turn_type IN ({})", types_str));
            }
        }

        if let Some(after) = self.started_after {
            conditions.push(format!(
                "started_at >= timestamp '{}'",
                after.format("%Y-%m-%d %H:%M:%S")
            ));
        }

        if let Some(before) = self.started_before {
            conditions.push(format!(
                "started_at < timestamp '{}'",
                before.format("%Y-%m-%d %H:%M:%S")
            ));
        }

        if conditions.is_empty() {
            None
        } else {
            Some(conditions.join(" AND "))
        }
    }
}

/// Filter options for session embedding searches.
#[derive(Debug, Clone, Default)]
pub struct SessionFilter {
    /// Filter by outcomes.
    pub outcomes: Option<Vec<String>>,

    /// Filter by providers.
    pub providers: Option<Vec<String>>,

    /// Filter by projects.
    pub projects: Option<Vec<String>>,

    /// Filter by sessions created after this time.
    pub created_after: Option<DateTime<Utc>>,

    /// Filter by sessions created before this time.
    pub created_before: Option<DateTime<Utc>>,
}

impl SessionFilter {
    /// Create a new empty filter.
    pub fn new() -> Self {
        Self::default()
    }

    /// Filter by outcomes.
    pub fn with_outcomes(mut self, outcomes: Vec<String>) -> Self {
        self.outcomes = Some(outcomes);
        self
    }

    /// Filter by providers.
    pub fn with_providers(mut self, providers: Vec<String>) -> Self {
        self.providers = Some(providers);
        self
    }

    /// Filter by projects.
    pub fn with_projects(mut self, projects: Vec<String>) -> Self {
        self.projects = Some(projects);
        self
    }

    /// Filter by time range.
    pub fn with_time_range(
        mut self,
        after: Option<DateTime<Utc>>,
        before: Option<DateTime<Utc>>,
    ) -> Self {
        self.created_after = after;
        self.created_before = before;
        self
    }

    /// Build a SQL WHERE clause for LanceDB.
    pub fn to_sql(&self) -> Option<String> {
        let mut conditions = Vec::new();

        if let Some(ref outcomes) = self.outcomes {
            if !outcomes.is_empty() {
                let outcomes_str = outcomes
                    .iter()
                    .map(|o| format!("'{}'", o))
                    .collect::<Vec<_>>()
                    .join(", ");
                conditions.push(format!("outcome IN ({})", outcomes_str));
            }
        }

        if let Some(ref providers) = self.providers {
            if !providers.is_empty() {
                let providers_str = providers
                    .iter()
                    .map(|p| format!("'{}'", p))
                    .collect::<Vec<_>>()
                    .join(", ");
                conditions.push(format!("provider IN ({})", providers_str));
            }
        }

        if let Some(ref projects) = self.projects {
            if !projects.is_empty() {
                let projects_str = projects
                    .iter()
                    .map(|p| format!("'{}'", p))
                    .collect::<Vec<_>>()
                    .join(", ");
                conditions.push(format!("project IN ({})", projects_str));
            }
        }

        if let Some(after) = self.created_after {
            conditions.push(format!(
                "created_at >= timestamp '{}'",
                after.format("%Y-%m-%d %H:%M:%S")
            ));
        }

        if let Some(before) = self.created_before {
            conditions.push(format!(
                "created_at < timestamp '{}'",
                before.format("%Y-%m-%d %H:%M:%S")
            ));
        }

        if conditions.is_empty() {
            None
        } else {
            Some(conditions.join(" AND "))
        }
    }
}

/// Statistics about the vector store.
#[derive(Debug, Clone, Default)]
pub struct VectorStoreStats {
    /// Number of turn embeddings.
    pub turn_count: usize,

    /// Number of session embeddings.
    pub session_count: usize,

    /// Embedding dimensions.
    pub dimensions: usize,

    /// Model name used for embeddings.
    pub model_name: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_turn_filter_to_sql_empty() {
        let filter = TurnFilter::new();
        assert!(filter.to_sql().is_none());
    }

    #[test]
    fn test_turn_filter_to_sql_session_id() {
        let filter = TurnFilter::new().with_session_id("abc123");
        assert_eq!(filter.to_sql().unwrap(), "session_id = 'abc123'");
    }

    #[test]
    fn test_turn_filter_to_sql_combined() {
        let filter = TurnFilter::new()
            .with_session_id("abc123")
            .with_turn_types(vec!["task".to_string(), "error_fix".to_string()]);

        let sql = filter.to_sql().unwrap();
        assert!(sql.contains("session_id = 'abc123'"));
        assert!(sql.contains("turn_type IN ('task', 'error_fix')"));
        assert!(sql.contains(" AND "));
    }

    #[test]
    fn test_session_filter_to_sql_providers() {
        let filter =
            SessionFilter::new().with_providers(vec!["claude".to_string(), "gemini".to_string()]);

        let sql = filter.to_sql().unwrap();
        assert!(sql.contains("provider IN ('claude', 'gemini')"));
    }
}
