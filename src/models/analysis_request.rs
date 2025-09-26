use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Status of an analysis request
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RequestStatus {
    /// Request is queued for processing
    Queued,
    /// Request is currently being processed
    Processing,
    /// Request completed successfully
    Completed,
    /// Request failed due to an error
    Failed,
}

impl std::fmt::Display for RequestStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RequestStatus::Queued => write!(f, "queued"),
            RequestStatus::Processing => write!(f, "processing"),
            RequestStatus::Completed => write!(f, "completed"),
            RequestStatus::Failed => write!(f, "failed"),
        }
    }
}

impl From<String> for RequestStatus {
    fn from(s: String) -> Self {
        match s.as_str() {
            "queued" => RequestStatus::Queued,
            "processing" => RequestStatus::Processing,
            "completed" => RequestStatus::Completed,
            "failed" => RequestStatus::Failed,
            _ => RequestStatus::Queued,
        }
    }
}

impl std::str::FromStr for RequestStatus {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "queued" => Ok(RequestStatus::Queued),
            "processing" => Ok(RequestStatus::Processing),
            "completed" => Ok(RequestStatus::Completed),
            "failed" => Ok(RequestStatus::Failed),
            _ => Ok(RequestStatus::Queued),
        }
    }
}

/// Represents a user's request to analyze specific chat sessions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisRequest {
    /// Unique request identifier
    pub id: Uuid,
    /// Target session for analysis
    pub session_id: Uuid,
    /// Template to use for analysis
    pub prompt_template_id: String,
    /// Variable values for template
    pub template_variables: HashMap<String, String>,
    /// Current request status
    pub status: RequestStatus,
    /// Error details if failed
    pub error_message: Option<String>,
    /// Request creation time
    pub created_at: DateTime<Utc>,
    /// Processing start time
    pub started_at: Option<DateTime<Utc>>,
    /// Processing completion time
    pub completed_at: Option<DateTime<Utc>>,
}

impl AnalysisRequest {
    /// Create a new analysis request
    pub fn new(
        session_id: Uuid,
        prompt_template_id: String,
        template_variables: HashMap<String, String>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            session_id,
            prompt_template_id,
            template_variables,
            status: RequestStatus::Queued,
            error_message: None,
            created_at: Utc::now(),
            started_at: None,
            completed_at: None,
        }
    }

    /// Create a request with a specific ID (for testing)
    pub fn with_id(
        id: Uuid,
        session_id: Uuid,
        prompt_template_id: String,
        template_variables: HashMap<String, String>,
    ) -> Self {
        Self {
            id,
            session_id,
            prompt_template_id,
            template_variables,
            status: RequestStatus::Queued,
            error_message: None,
            created_at: Utc::now(),
            started_at: None,
            completed_at: None,
        }
    }

    /// Start processing the request
    pub fn start_processing(&mut self) {
        self.status = RequestStatus::Processing;
        self.started_at = Some(Utc::now());
        self.error_message = None;
    }

    /// Mark request as completed successfully
    pub fn complete(&mut self) {
        self.status = RequestStatus::Completed;
        self.completed_at = Some(Utc::now());
        self.error_message = None;
    }

    /// Mark request as failed with error message
    pub fn fail(&mut self, error_message: String) {
        self.status = RequestStatus::Failed;
        self.completed_at = Some(Utc::now());
        self.error_message = Some(error_message);
    }

    /// Retry a failed request (reset to queued state)
    pub fn retry(&mut self) {
        if matches!(self.status, RequestStatus::Failed) {
            self.status = RequestStatus::Queued;
            self.started_at = None;
            self.completed_at = None;
            self.error_message = None;
        }
    }

    /// Check if request is in a terminal state (completed or failed)
    pub fn is_terminal(&self) -> bool {
        matches!(
            self.status,
            RequestStatus::Completed | RequestStatus::Failed
        )
    }

    /// Check if request is currently being processed
    pub fn is_processing(&self) -> bool {
        matches!(self.status, RequestStatus::Processing)
    }

    /// Check if request is queued for processing
    pub fn is_queued(&self) -> bool {
        matches!(self.status, RequestStatus::Queued)
    }

    /// Check if request completed successfully
    pub fn is_completed(&self) -> bool {
        matches!(self.status, RequestStatus::Completed)
    }

    /// Check if request failed
    pub fn is_failed(&self) -> bool {
        matches!(self.status, RequestStatus::Failed)
    }

    /// Get processing duration if available
    pub fn get_processing_duration(&self) -> Option<chrono::Duration> {
        if let (Some(started), Some(completed)) = (self.started_at, self.completed_at) {
            Some(completed - started)
        } else if let Some(started) = self.started_at {
            Some(Utc::now() - started)
        } else {
            None
        }
    }

    /// Get total duration from creation to completion
    pub fn get_total_duration(&self) -> chrono::Duration {
        let end_time = self.completed_at.unwrap_or_else(Utc::now);
        end_time - self.created_at
    }

    /// Get age of the request
    pub fn get_age(&self) -> chrono::Duration {
        Utc::now() - self.created_at
    }

    /// Add or update a template variable
    pub fn set_variable(&mut self, name: String, value: String) {
        self.template_variables.insert(name, value);
    }

    /// Remove a template variable
    pub fn remove_variable(&mut self, name: &str) {
        self.template_variables.remove(name);
    }

    /// Get a template variable value
    pub fn get_variable(&self, name: &str) -> Option<&String> {
        self.template_variables.get(name)
    }

    /// Check if all required variables are provided (requires template for validation)
    pub fn has_required_variables(&self, required_vars: &[String]) -> bool {
        required_vars
            .iter()
            .all(|var| self.template_variables.contains_key(var))
    }

    /// Get a priority score for queue ordering (lower is higher priority)
    pub fn get_priority_score(&self) -> i64 {
        // Base priority on age - older requests have higher priority
        let age_minutes = self.get_age().num_minutes();

        // Failed requests that are retried get higher priority
        let retry_bonus = if self.error_message.is_some() {
            -100
        } else {
            0
        };

        age_minutes + retry_bonus
    }

    /// Check if request should be considered stale (abandoned)
    pub fn is_stale(&self, max_age_hours: i64) -> bool {
        let max_age = chrono::Duration::hours(max_age_hours);
        match self.status {
            RequestStatus::Queued => self.get_age() > max_age,
            RequestStatus::Processing => {
                if let Some(started) = self.started_at {
                    (Utc::now() - started) > chrono::Duration::hours(1) // 1 hour processing timeout
                } else {
                    false
                }
            }
            _ => false, // Terminal states are not stale
        }
    }

    /// Create a summary for display
    pub fn get_summary(&self) -> String {
        format!(
            "Request {} for session {} using template '{}' - Status: {}",
            self.id.to_string()[..8].to_uppercase(),
            self.session_id.to_string()[..8].to_uppercase(),
            self.prompt_template_id,
            self.status
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_request_status_conversion() {
        assert_eq!(
            RequestStatus::from("queued".to_string()),
            RequestStatus::Queued
        );
        assert_eq!(
            RequestStatus::from("processing".to_string()),
            RequestStatus::Processing
        );
        assert_eq!(
            RequestStatus::from("completed".to_string()),
            RequestStatus::Completed
        );
        assert_eq!(
            RequestStatus::from("failed".to_string()),
            RequestStatus::Failed
        );
        assert_eq!(
            RequestStatus::from("invalid".to_string()),
            RequestStatus::Queued
        );
    }

    #[test]
    fn test_request_status_display() {
        assert_eq!(RequestStatus::Queued.to_string(), "queued");
        assert_eq!(RequestStatus::Processing.to_string(), "processing");
        assert_eq!(RequestStatus::Completed.to_string(), "completed");
        assert_eq!(RequestStatus::Failed.to_string(), "failed");
    }

    #[test]
    fn test_new_analysis_request() {
        let session_id = Uuid::new_v4();
        let template_id = "test_template".to_string();
        let mut variables = HashMap::new();
        variables.insert("chat_content".to_string(), "test content".to_string());

        let request = AnalysisRequest::new(session_id, template_id.clone(), variables.clone());

        assert_eq!(request.session_id, session_id);
        assert_eq!(request.prompt_template_id, template_id);
        assert_eq!(request.template_variables, variables);
        assert_eq!(request.status, RequestStatus::Queued);
        assert!(request.is_queued());
        assert!(!request.is_terminal());
    }

    #[test]
    fn test_request_lifecycle() {
        let mut request =
            AnalysisRequest::new(Uuid::new_v4(), "test_template".to_string(), HashMap::new());

        // Start processing
        request.start_processing();
        assert_eq!(request.status, RequestStatus::Processing);
        assert!(request.is_processing());
        assert!(request.started_at.is_some());

        // Complete successfully
        request.complete();
        assert_eq!(request.status, RequestStatus::Completed);
        assert!(request.is_completed());
        assert!(request.is_terminal());
        assert!(request.completed_at.is_some());
    }

    #[test]
    fn test_request_failure_and_retry() {
        let mut request =
            AnalysisRequest::new(Uuid::new_v4(), "test_template".to_string(), HashMap::new());

        // Fail the request
        request.fail("API error".to_string());
        assert_eq!(request.status, RequestStatus::Failed);
        assert!(request.is_failed());
        assert!(request.is_terminal());
        assert_eq!(request.error_message, Some("API error".to_string()));

        // Retry the request
        request.retry();
        assert_eq!(request.status, RequestStatus::Queued);
        assert!(request.is_queued());
        assert!(!request.is_terminal());
        assert!(request.error_message.is_none());
    }

    #[test]
    fn test_duration_calculations() {
        let mut request =
            AnalysisRequest::new(Uuid::new_v4(), "test_template".to_string(), HashMap::new());

        // Initially no processing duration
        assert!(request.get_processing_duration().is_none());

        // Start processing
        request.start_processing();
        thread::sleep(Duration::from_millis(10));

        // Should have some processing duration
        let duration = request.get_processing_duration();
        assert!(duration.is_some());
        assert!(duration.unwrap().num_milliseconds() >= 0);

        // Complete and check total duration
        request.complete();
        let total_duration = request.get_total_duration();
        assert!(total_duration.num_milliseconds() > 0);
    }

    #[test]
    fn test_variable_management() {
        let mut request =
            AnalysisRequest::new(Uuid::new_v4(), "test_template".to_string(), HashMap::new());

        // Add variables
        request.set_variable("var1".to_string(), "value1".to_string());
        request.set_variable("var2".to_string(), "value2".to_string());

        assert_eq!(request.get_variable("var1"), Some(&"value1".to_string()));
        assert_eq!(request.get_variable("var2"), Some(&"value2".to_string()));
        assert_eq!(request.get_variable("nonexistent"), None);

        // Update variable
        request.set_variable("var1".to_string(), "updated_value".to_string());
        assert_eq!(
            request.get_variable("var1"),
            Some(&"updated_value".to_string())
        );

        // Remove variable
        request.remove_variable("var2");
        assert_eq!(request.get_variable("var2"), None);

        // Check required variables
        let required_vars = vec!["var1".to_string(), "var3".to_string()];
        assert!(!request.has_required_variables(&required_vars));

        request.set_variable("var3".to_string(), "value3".to_string());
        assert!(request.has_required_variables(&required_vars));
    }

    #[test]
    fn test_priority_and_staleness() {
        let mut old_request =
            AnalysisRequest::new(Uuid::new_v4(), "test_template".to_string(), HashMap::new());

        let new_request =
            AnalysisRequest::new(Uuid::new_v4(), "test_template".to_string(), HashMap::new());

        // Old request should have higher priority (lower score)
        thread::sleep(Duration::from_millis(10));
        assert!(old_request.get_priority_score() >= new_request.get_priority_score());

        // Failed requests get priority boost
        old_request.fail("Test error".to_string());
        old_request.retry();
        assert!(old_request.get_priority_score() < new_request.get_priority_score());

        // Test staleness (using very small threshold for test)
        assert!(!new_request.is_stale(24)); // Not stale within 24 hours
        assert!(old_request.is_stale(0)); // Stale if max age is 0 hours
    }

    #[test]
    fn test_summary_generation() {
        let session_id = Uuid::new_v4();
        let request = AnalysisRequest::new(session_id, "test_template".to_string(), HashMap::new());

        let summary = request.get_summary();
        assert!(summary.contains(&request.id.to_string()[..8].to_uppercase()));
        assert!(summary.contains(&session_id.to_string()[..8].to_uppercase()));
        assert!(summary.contains("test_template"));
        assert!(summary.contains("queued"));
    }
}
