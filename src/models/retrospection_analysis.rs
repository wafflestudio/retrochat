use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::analysis_metadata::AnalysisMetadata;

/// Status of a retrospection analysis
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AnalysisStatus {
    /// Analysis is in draft state (not yet started)
    Draft,
    /// Analysis is currently being processed
    InProgress,
    /// Analysis completed successfully
    Complete,
    /// Analysis failed due to an error
    Failed,
    /// Analysis has been archived
    Archived,
}

impl std::fmt::Display for AnalysisStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AnalysisStatus::Draft => write!(f, "draft"),
            AnalysisStatus::InProgress => write!(f, "in_progress"),
            AnalysisStatus::Complete => write!(f, "complete"),
            AnalysisStatus::Failed => write!(f, "failed"),
            AnalysisStatus::Archived => write!(f, "archived"),
        }
    }
}

impl From<String> for AnalysisStatus {
    fn from(s: String) -> Self {
        match s.as_str() {
            "draft" => AnalysisStatus::Draft,
            "in_progress" => AnalysisStatus::InProgress,
            "complete" => AnalysisStatus::Complete,
            "failed" => AnalysisStatus::Failed,
            "archived" => AnalysisStatus::Archived,
            _ => AnalysisStatus::Draft,
        }
    }
}

impl std::str::FromStr for AnalysisStatus {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "draft" => Ok(AnalysisStatus::Draft),
            "in_progress" => Ok(AnalysisStatus::InProgress),
            "complete" => Ok(AnalysisStatus::Complete),
            "failed" => Ok(AnalysisStatus::Failed),
            "archived" => Ok(AnalysisStatus::Archived),
            _ => Ok(AnalysisStatus::Draft),
        }
    }
}

/// Represents the output from LLM analysis of chat sessions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrospectionAnalysis {
    /// Unique identifier for the analysis
    pub id: Uuid,
    /// Foreign key to chat_sessions table
    pub session_id: Uuid,
    /// ID of the prompt template used
    pub prompt_template_id: String,
    /// Full LLM response text
    pub analysis_content: String,
    /// Analysis execution details
    pub metadata: AnalysisMetadata,
    /// Current status of the analysis
    pub status: AnalysisStatus,
    /// When analysis was performed
    pub created_at: DateTime<Utc>,
    /// Last modification time
    pub updated_at: DateTime<Utc>,
}

impl RetrospectionAnalysis {
    /// Create a new pending analysis
    pub fn new(session_id: Uuid, prompt_template_id: String) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            session_id,
            prompt_template_id,
            analysis_content: String::new(),
            metadata: AnalysisMetadata::default(),
            status: AnalysisStatus::Draft,
            created_at: now,
            updated_at: now,
        }
    }

    /// Mark analysis as in progress
    pub fn start_processing(&mut self) {
        self.status = AnalysisStatus::InProgress;
        self.updated_at = Utc::now();
    }

    /// Complete the analysis with content and metadata
    pub fn complete(&mut self, content: String, metadata: AnalysisMetadata) {
        self.analysis_content = content;
        self.metadata = metadata;
        self.status = AnalysisStatus::Complete;
        self.updated_at = Utc::now();
    }

    /// Mark analysis as failed with error message
    pub fn fail(&mut self, error_message: &str) {
        self.status = AnalysisStatus::Failed;
        self.analysis_content = format!("Analysis failed: {}", error_message);
        self.updated_at = Utc::now();
    }

    /// Check if analysis is completed successfully
    pub fn is_completed(&self) -> bool {
        matches!(self.status, AnalysisStatus::Complete)
    }

    /// Check if analysis failed
    pub fn is_failed(&self) -> bool {
        matches!(self.status, AnalysisStatus::Failed)
    }

    /// Check if analysis is still processing
    pub fn is_processing(&self) -> bool {
        matches!(self.status, AnalysisStatus::InProgress)
    }

    /// Get a summary of the analysis for display
    pub fn get_summary(&self) -> String {
        if self.analysis_content.len() <= 200 {
            self.analysis_content.clone()
        } else {
            format!("{}...", &self.analysis_content[..197])
        }
    }

    /// Get the estimated cost in USD
    pub fn get_estimated_cost(&self) -> f64 {
        self.metadata.estimated_cost
    }

    /// Get the execution time in seconds
    pub fn get_execution_time_seconds(&self) -> f64 {
        self.metadata.execution_time_ms as f64 / 1000.0
    }

    /// Get the total token count
    pub fn get_total_tokens(&self) -> u32 {
        self.metadata.total_tokens
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_analysis_status_conversion() {
        assert_eq!(
            AnalysisStatus::from("draft".to_string()),
            AnalysisStatus::Draft
        );
        assert_eq!(
            AnalysisStatus::from("complete".to_string()),
            AnalysisStatus::Complete
        );
        assert_eq!(
            AnalysisStatus::from("invalid".to_string()),
            AnalysisStatus::Draft
        );
    }

    #[test]
    fn test_analysis_status_display() {
        assert_eq!(AnalysisStatus::Draft.to_string(), "draft");
        assert_eq!(AnalysisStatus::Complete.to_string(), "complete");
        assert_eq!(AnalysisStatus::Failed.to_string(), "failed");
    }

    #[test]
    fn test_new_analysis() {
        let session_id = Uuid::new_v4();
        let template_id = "test_template".to_string();

        let analysis = RetrospectionAnalysis::new(session_id, template_id.clone());

        assert_eq!(analysis.session_id, session_id);
        assert_eq!(analysis.prompt_template_id, template_id);
        assert_eq!(analysis.status, AnalysisStatus::Draft);
        assert!(analysis.analysis_content.is_empty());
        assert!(!analysis.is_processing());
        assert!(!analysis.is_completed());
    }

    #[test]
    fn test_analysis_lifecycle() {
        let mut analysis = RetrospectionAnalysis::new(Uuid::new_v4(), "test_template".to_string());

        // Start processing
        analysis.start_processing();
        assert_eq!(analysis.status, AnalysisStatus::InProgress);
        assert!(analysis.is_processing());

        // Complete analysis
        let metadata = AnalysisMetadata {
            llm_service: "gemini-2.5-flash-lite".to_string(),
            prompt_tokens: 100,
            completion_tokens: 50,
            total_tokens: 150,
            estimated_cost: 0.001,
            execution_time_ms: 1500,
            api_response_metadata: None,
        };

        analysis.complete("Test analysis result".to_string(), metadata);
        assert_eq!(analysis.status, AnalysisStatus::Complete);
        assert!(analysis.is_completed());
        assert!(!analysis.is_processing());
        assert_eq!(analysis.analysis_content, "Test analysis result");
    }

    #[test]
    fn test_analysis_failure() {
        let mut analysis = RetrospectionAnalysis::new(Uuid::new_v4(), "test_template".to_string());

        analysis.fail("API rate limit exceeded");
        assert_eq!(analysis.status, AnalysisStatus::Failed);
        assert!(analysis.is_failed());
        assert!(analysis
            .analysis_content
            .contains("API rate limit exceeded"));
    }

    #[test]
    fn test_analysis_summary() {
        let mut analysis = RetrospectionAnalysis::new(Uuid::new_v4(), "test_template".to_string());

        // Short content
        analysis.analysis_content = "Short content".to_string();
        assert_eq!(analysis.get_summary(), "Short content");

        // Long content
        analysis.analysis_content = "a".repeat(300);
        let summary = analysis.get_summary();
        assert_eq!(summary.len(), 200);
        assert!(summary.ends_with("..."));
    }

    #[test]
    fn test_metrics_getters() {
        let metadata = AnalysisMetadata {
            llm_service: "gemini-2.5-flash-lite".to_string(),
            prompt_tokens: 100,
            completion_tokens: 50,
            total_tokens: 150,
            estimated_cost: 0.0025,
            execution_time_ms: 2500,
            api_response_metadata: None,
        };

        let mut analysis = RetrospectionAnalysis::new(Uuid::new_v4(), "test_template".to_string());

        analysis.complete("Test content".to_string(), metadata);

        assert_eq!(analysis.get_total_tokens(), 150);
        assert_eq!(analysis.get_estimated_cost(), 0.0025);
        assert_eq!(analysis.get_execution_time_seconds(), 2.5);
    }
}
