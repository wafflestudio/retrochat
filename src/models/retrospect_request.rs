use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RetrospectionAnalysisType {
    UserInteractionAnalysis,
    CollaborationInsights,
    QuestionQuality,
    TaskBreakdown,
    FollowUpPatterns,
    Custom(String),
}

impl std::fmt::Display for RetrospectionAnalysisType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RetrospectionAnalysisType::UserInteractionAnalysis => write!(f, "user-interaction"),
            RetrospectionAnalysisType::CollaborationInsights => write!(f, "collaboration"),
            RetrospectionAnalysisType::QuestionQuality => write!(f, "question-quality"),
            RetrospectionAnalysisType::TaskBreakdown => write!(f, "task-breakdown"),
            RetrospectionAnalysisType::FollowUpPatterns => write!(f, "follow-up"),
            RetrospectionAnalysisType::Custom(_) => write!(f, "custom"),
        }
    }
}

impl std::str::FromStr for RetrospectionAnalysisType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "user-interaction" => Ok(RetrospectionAnalysisType::UserInteractionAnalysis),
            "collaboration" => Ok(RetrospectionAnalysisType::CollaborationInsights),
            "question-quality" => Ok(RetrospectionAnalysisType::QuestionQuality),
            "task-breakdown" => Ok(RetrospectionAnalysisType::TaskBreakdown),
            "follow-up" => Ok(RetrospectionAnalysisType::FollowUpPatterns),
            s if s.starts_with("custom:") => {
                Ok(RetrospectionAnalysisType::Custom(s[7..].to_string()))
            }
            _ => Err(format!("Invalid analysis type: {}", s)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum OperationStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

impl std::fmt::Display for OperationStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OperationStatus::Pending => write!(f, "pending"),
            OperationStatus::Running => write!(f, "running"),
            OperationStatus::Completed => write!(f, "completed"),
            OperationStatus::Failed => write!(f, "failed"),
            OperationStatus::Cancelled => write!(f, "cancelled"),
        }
    }
}

impl std::str::FromStr for OperationStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "pending" => Ok(OperationStatus::Pending),
            "running" => Ok(OperationStatus::Running),
            "completed" => Ok(OperationStatus::Completed),
            "failed" => Ok(OperationStatus::Failed),
            "cancelled" => Ok(OperationStatus::Cancelled),
            _ => Err(format!("Invalid operation status: {}", s)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrospectRequest {
    pub id: String,
    pub session_id: String,
    pub analysis_type: RetrospectionAnalysisType,
    pub status: OperationStatus,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub created_by: Option<String>,
    pub error_message: Option<String>,
    pub custom_prompt: Option<String>,
}

impl RetrospectRequest {
    pub fn new(
        session_id: String,
        analysis_type: RetrospectionAnalysisType,
        created_by: Option<String>,
        custom_prompt: Option<String>,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            session_id,
            analysis_type,
            status: OperationStatus::Pending,
            started_at: Utc::now(),
            completed_at: None,
            created_by,
            error_message: None,
            custom_prompt,
        }
    }

    pub fn is_active(&self) -> bool {
        matches!(self.status, OperationStatus::Pending | OperationStatus::Running)
    }

    pub fn is_completed(&self) -> bool {
        matches!(
            self.status,
            OperationStatus::Completed | OperationStatus::Failed | OperationStatus::Cancelled
        )
    }

    pub fn mark_running(&mut self) {
        self.status = OperationStatus::Running;
    }

    pub fn mark_completed(&mut self) {
        self.status = OperationStatus::Completed;
        self.completed_at = Some(Utc::now());
    }

    pub fn mark_failed(&mut self, error_message: String) {
        self.status = OperationStatus::Failed;
        self.completed_at = Some(Utc::now());
        self.error_message = Some(error_message);
    }

    pub fn mark_cancelled(&mut self) {
        self.status = OperationStatus::Cancelled;
        self.completed_at = Some(Utc::now());
    }

    pub fn mark_pending(&mut self) {
        self.status = OperationStatus::Pending;
        self.completed_at = None;
        self.error_message = None;
        self.started_at = Utc::now();
    }

    pub fn duration(&self) -> Option<chrono::Duration> {
        self.completed_at
            .map(|completed| completed - self.started_at)
    }
}

#[derive(Debug, Clone)]
pub struct RetrospectionRequest {
    pub session_id: String,
    pub analysis_type: RetrospectionAnalysisType,
    pub custom_prompt: Option<String>,
    pub user_id: Option<String>,
}

impl RetrospectionRequest {
    pub fn new(
        session_id: String,
        analysis_type: RetrospectionAnalysisType,
        user_id: Option<String>,
    ) -> Self {
        Self {
            session_id,
            analysis_type,
            custom_prompt: None,
            user_id,
        }
    }

    pub fn with_custom_prompt(mut self, prompt: String) -> Self {
        self.custom_prompt = Some(prompt.clone());
        if matches!(self.analysis_type, RetrospectionAnalysisType::Custom(_)) {
            self.analysis_type = RetrospectionAnalysisType::Custom(prompt);
        }
        self
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.session_id.is_empty() {
            return Err("Session ID cannot be empty".to_string());
        }

        if let RetrospectionAnalysisType::Custom(ref prompt) = self.analysis_type {
            if prompt.is_empty() && self.custom_prompt.is_none() {
                return Err("Custom prompt is required for custom analysis type".to_string());
            }
        }

        Ok(())
    }
}
