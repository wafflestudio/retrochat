use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

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
            _ => Err(format!("Invalid operation status: {s}")),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyticsRequest {
    pub id: String,
    pub session_id: String,
    pub status: OperationStatus,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub created_by: Option<String>,
    pub error_message: Option<String>,
    pub custom_prompt: Option<String>,
}

impl AnalyticsRequest {
    pub fn new(
        session_id: String,
        created_by: Option<String>,
        custom_prompt: Option<String>,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            session_id,
            status: OperationStatus::Pending,
            started_at: Utc::now(),
            completed_at: None,
            created_by,
            error_message: None,
            custom_prompt,
        }
    }

    pub fn is_active(&self) -> bool {
        matches!(
            self.status,
            OperationStatus::Pending | OperationStatus::Running
        )
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