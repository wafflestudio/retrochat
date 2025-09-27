use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, RwLock};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;
use chrono::{DateTime, Utc};

use crate::database::DatabaseManager;
use crate::models::OperationStatus;

#[derive(Debug, Clone)]
pub struct BackgroundOperation {
    pub id: String,
    pub operation_type: String,
    pub description: String,
    pub status: OperationStatus,
    pub progress_percentage: Option<u8>,
    pub message: Option<String>,
    pub error_message: Option<String>,
    pub started_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub created_by: Option<String>,
    pub cancellation_token: CancellationToken,
}

impl BackgroundOperation {
    pub fn new(
        operation_type: String,
        description: String,
        created_by: Option<String>,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            operation_type,
            description,
            status: OperationStatus::Pending,
            progress_percentage: None,
            message: None,
            error_message: None,
            started_at: now,
            updated_at: now,
            completed_at: None,
            created_by,
            cancellation_token: CancellationToken::new(),
        }
    }

    pub fn is_active(&self) -> bool {
        matches!(self.status, OperationStatus::Pending | OperationStatus::Running)
    }

    pub fn can_be_cancelled(&self) -> bool {
        self.is_active()
    }

    pub fn update_progress(&mut self, percentage: u8, message: Option<String>) {
        self.progress_percentage = Some(percentage.min(100));
        self.message = message;
        self.updated_at = Utc::now();

        if percentage >= 100 && self.status != OperationStatus::Completed {
            self.mark_completed(None);
        } else if self.status == OperationStatus::Pending {
            self.status = OperationStatus::Running;
        }
    }

    pub fn mark_running(&mut self) {
        self.status = OperationStatus::Running;
        self.updated_at = Utc::now();
    }

    pub fn mark_completed(&mut self, message: Option<String>) {
        self.status = OperationStatus::Completed;
        self.completed_at = Some(Utc::now());
        self.updated_at = self.completed_at.unwrap();
        self.progress_percentage = Some(100);
        if let Some(msg) = message {
            self.message = Some(msg);
        }
    }

    pub fn mark_failed(&mut self, error_message: String) {
        self.status = OperationStatus::Failed;
        self.completed_at = Some(Utc::now());
        self.updated_at = self.completed_at.unwrap();
        self.error_message = Some(error_message);
    }

    pub fn mark_cancelled(&mut self) {
        self.status = OperationStatus::Cancelled;
        self.completed_at = Some(Utc::now());
        self.updated_at = self.completed_at.unwrap();
        self.cancellation_token.cancel();
    }

    pub fn duration(&self) -> Duration {
        let end_time = self.completed_at.unwrap_or_else(Utc::now);
        (end_time - self.started_at).to_std().unwrap_or(Duration::ZERO)
    }
}

#[derive(Debug)]
pub struct OperationResult {
    pub operation_id: String,
    pub success: bool,
    pub message: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug)]
pub struct CancellationResult {
    pub success: bool,
    pub total_cancelled: usize,
    pub force_cancelled: usize,
    pub graceful_cancelled: usize,
    pub errors: Vec<String>,
}

#[derive(Debug)]
pub struct OperationUpdate {
    pub operation_id: String,
    pub status: OperationStatus,
    pub progress_percentage: Option<u8>,
    pub message: Option<String>,
    pub error: Option<String>,
}

type OperationUpdateSender = mpsc::UnboundedSender<OperationUpdate>;
type OperationUpdateReceiver = mpsc::UnboundedReceiver<OperationUpdate>;

#[derive(Clone)]
pub struct BackgroundOperationManager {
    db_manager: Arc<DatabaseManager>,
    operations: Arc<RwLock<HashMap<String, BackgroundOperation>>>,
    update_sender: OperationUpdateSender,
    cleanup_interval: Duration,
}

impl BackgroundOperationManager {
    pub fn new(db_manager: Arc<DatabaseManager>) -> Self {
        let (update_sender, _update_receiver) = mpsc::unbounded_channel();

        Self {
            db_manager,
            operations: Arc::new(RwLock::new(HashMap::new())),
            update_sender,
            cleanup_interval: Duration::from_secs(3600), // 1 hour
        }
    }

    pub fn with_cleanup_interval(mut self, interval: Duration) -> Self {
        self.cleanup_interval = interval;
        self
    }

    pub async fn start_operation(
        &self,
        operation_type: String,
        description: String,
        created_by: Option<String>,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let operation = BackgroundOperation::new(operation_type, description, created_by);
        let operation_id = operation.id.clone();

        // Store in memory
        {
            let mut operations = self.operations.write().await;
            operations.insert(operation_id.clone(), operation.clone());
        }

        // Persist to database
        self.persist_operation(&operation).await?;

        // Send update notification
        let _ = self.update_sender.send(OperationUpdate {
            operation_id: operation_id.clone(),
            status: operation.status,
            progress_percentage: operation.progress_percentage,
            message: operation.message.clone(),
            error: operation.error_message.clone(),
        });

        Ok(operation_id)
    }

    pub async fn get_operation(
        &self,
        operation_id: &str,
    ) -> Result<BackgroundOperation, Box<dyn std::error::Error + Send + Sync>> {
        // Try memory first
        {
            let operations = self.operations.read().await;
            if let Some(operation) = operations.get(operation_id) {
                return Ok(operation.clone());
            }
        }

        // Fall back to database
        self.load_operation_from_db(operation_id).await
    }

    pub async fn update_operation_status(
        &self,
        operation_id: &str,
        status: OperationStatus,
        message: Option<String>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let updated_operation = {
            let mut operations = self.operations.write().await;
            if let Some(operation) = operations.get_mut(operation_id) {
                operation.status = status.clone();
                operation.message = message.clone();
                operation.updated_at = Utc::now();

                if matches!(status, OperationStatus::Completed | OperationStatus::Failed | OperationStatus::Cancelled) {
                    operation.completed_at = Some(operation.updated_at);
                }

                operation.clone()
            } else {
                return Err("Operation not found".into());
            }
        };

        // Persist to database
        self.persist_operation(&updated_operation).await?;

        // Send update notification
        let _ = self.update_sender.send(OperationUpdate {
            operation_id: operation_id.to_string(),
            status,
            progress_percentage: updated_operation.progress_percentage,
            message,
            error: updated_operation.error_message,
        });

        Ok(())
    }

    pub async fn update_operation_progress(
        &self,
        operation_id: &str,
        progress: u8,
        message: Option<String>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let updated_operation = {
            let mut operations = self.operations.write().await;
            if let Some(operation) = operations.get_mut(operation_id) {
                operation.update_progress(progress, message.clone());
                operation.clone()
            } else {
                return Err("Operation not found".into());
            }
        };

        // Persist to database
        self.persist_operation(&updated_operation).await?;

        // Send update notification
        let _ = self.update_sender.send(OperationUpdate {
            operation_id: operation_id.to_string(),
            status: updated_operation.status,
            progress_percentage: Some(progress),
            message,
            error: None,
        });

        Ok(())
    }

    pub async fn complete_operation(
        &self,
        operation_id: &str,
        success: bool,
        message: Option<String>,
    ) -> Result<OperationResult, Box<dyn std::error::Error + Send + Sync>> {
        let updated_operation = {
            let mut operations = self.operations.write().await;
            if let Some(operation) = operations.get_mut(operation_id) {
                if success {
                    operation.mark_completed(message.clone());
                } else {
                    operation.mark_failed(message.clone().unwrap_or_else(|| "Operation failed".to_string()));
                }
                operation.clone()
            } else {
                return Err("Operation not found".into());
            }
        };

        // Persist to database
        self.persist_operation(&updated_operation).await?;

        // Send update notification
        let _ = self.update_sender.send(OperationUpdate {
            operation_id: operation_id.to_string(),
            status: updated_operation.status.clone(),
            progress_percentage: updated_operation.progress_percentage,
            message: message.clone(),
            error: if success { None } else { message.clone() },
        });

        Ok(OperationResult {
            operation_id: operation_id.to_string(),
            success,
            message: message.clone(),
            error: if success { None } else { message },
        })
    }

    pub async fn fail_operation(
        &self,
        operation_id: &str,
        error_message: String,
        details: Option<String>,
    ) -> Result<OperationResult, Box<dyn std::error::Error + Send + Sync>> {
        let final_error = if let Some(details) = details {
            format!("{}: {}", error_message, details)
        } else {
            error_message.clone()
        };

        let updated_operation = {
            let mut operations = self.operations.write().await;
            if let Some(operation) = operations.get_mut(operation_id) {
                operation.mark_failed(final_error.clone());
                operation.clone()
            } else {
                return Err("Operation not found".into());
            }
        };

        // Persist to database
        self.persist_operation(&updated_operation).await?;

        // Send update notification
        let _ = self.update_sender.send(OperationUpdate {
            operation_id: operation_id.to_string(),
            status: OperationStatus::Failed,
            progress_percentage: updated_operation.progress_percentage,
            message: None,
            error: Some(final_error.clone()),
        });

        Ok(OperationResult {
            operation_id: operation_id.to_string(),
            success: false,
            message: None,
            error: Some(final_error),
        })
    }

    pub async fn cancel_operation(
        &self,
        operation_id: &str,
    ) -> Result<OperationResult, Box<dyn std::error::Error + Send + Sync>> {
        let updated_operation = {
            let mut operations = self.operations.write().await;
            if let Some(operation) = operations.get_mut(operation_id) {
                if !operation.can_be_cancelled() {
                    return Ok(OperationResult {
                        operation_id: operation_id.to_string(),
                        success: false,
                        message: None,
                        error: Some("Operation cannot be cancelled".to_string()),
                    });
                }

                operation.mark_cancelled();
                operation.clone()
            } else {
                return Err("Operation not found".into());
            }
        };

        // Persist to database
        self.persist_operation(&updated_operation).await?;

        // Send update notification
        let _ = self.update_sender.send(OperationUpdate {
            operation_id: operation_id.to_string(),
            status: OperationStatus::Cancelled,
            progress_percentage: updated_operation.progress_percentage,
            message: Some("Operation cancelled by user".to_string()),
            error: None,
        });

        Ok(OperationResult {
            operation_id: operation_id.to_string(),
            success: true,
            message: Some("Operation cancelled successfully".to_string()),
            error: None,
        })
    }

    pub async fn cancel_all_operations(
        &self,
        created_by: Option<String>,
    ) -> Result<CancellationResult, Box<dyn std::error::Error + Send + Sync>> {
        let operations_to_cancel: Vec<String> = {
            let operations = self.operations.read().await;
            operations
                .values()
                .filter(|op| {
                    op.can_be_cancelled() &&
                    (created_by.is_none() || op.created_by == created_by)
                })
                .map(|op| op.id.clone())
                .collect()
        };

        let mut total_cancelled = 0;
        let mut graceful_cancelled = 0;
        let mut errors = Vec::new();

        for operation_id in operations_to_cancel {
            match self.cancel_operation(&operation_id).await {
                Ok(result) => {
                    if result.success {
                        total_cancelled += 1;
                        graceful_cancelled += 1;
                    } else {
                        errors.push(format!("Failed to cancel {}: {}", operation_id,
                            result.error.unwrap_or_else(|| "Unknown error".to_string())));
                    }
                }
                Err(e) => {
                    errors.push(format!("Error cancelling {}: {}", operation_id, e));
                }
            }
        }

        Ok(CancellationResult {
            success: total_cancelled > 0,
            total_cancelled,
            force_cancelled: 0, // We don't implement force cancellation yet
            graceful_cancelled,
            errors,
        })
    }

    pub async fn get_active_operations(
        &self,
    ) -> Result<Vec<BackgroundOperation>, Box<dyn std::error::Error + Send + Sync>> {
        let operations = self.operations.read().await;
        Ok(operations
            .values()
            .filter(|op| op.is_active())
            .cloned()
            .collect())
    }

    pub async fn get_recent_operations(
        &self,
        limit: Option<usize>,
    ) -> Result<Vec<BackgroundOperation>, Box<dyn std::error::Error + Send + Sync>> {
        let operations = self.operations.read().await;
        let mut recent: Vec<_> = operations
            .values()
            .filter(|op| !op.is_active())
            .cloned()
            .collect();

        recent.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));

        if let Some(limit) = limit {
            recent.truncate(limit);
        }

        Ok(recent)
    }

    pub async fn cleanup_old_operations(
        &self,
        older_than: Duration,
    ) -> Result<usize, Box<dyn std::error::Error + Send + Sync>> {
        let cutoff_time = Utc::now() - chrono::Duration::from_std(older_than)?;
        let mut cleaned_count = 0;

        // Remove from memory
        {
            let mut operations = self.operations.write().await;
            let to_remove: Vec<String> = operations
                .values()
                .filter(|op| {
                    !op.is_active() && op.updated_at < cutoff_time
                })
                .map(|op| op.id.clone())
                .collect();

            for operation_id in to_remove {
                operations.remove(&operation_id);
                cleaned_count += 1;
            }
        }

        // Clean from database
        self.cleanup_database_operations(cutoff_time).await?;

        Ok(cleaned_count)
    }

    pub fn subscribe_to_updates(&self) -> OperationUpdateReceiver {
        let (_sender, receiver) = mpsc::unbounded_channel();
        receiver
    }

    async fn persist_operation(
        &self,
        operation: &BackgroundOperation,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // This would implement database persistence
        // For now, we'll just log it
        tracing::debug!("Persisting operation {}: {:?}", operation.id, operation);
        Ok(())
    }

    async fn load_operation_from_db(
        &self,
        operation_id: &str,
    ) -> Result<BackgroundOperation, Box<dyn std::error::Error + Send + Sync>> {
        // This would implement database loading
        // For now, we'll return an error
        Err(format!("Operation {} not found in database", operation_id).into())
    }

    async fn cleanup_database_operations(
        &self,
        cutoff_time: DateTime<Utc>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // This would implement database cleanup
        tracing::debug!("Cleaning up operations older than {}", cutoff_time);
        Ok(())
    }

    pub async fn start_cleanup_task(self: Arc<Self>) {
        let manager = self.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(manager.cleanup_interval);
            loop {
                interval.tick().await;
                if let Err(e) = manager.cleanup_old_operations(Duration::from_secs(86400)).await {
                    tracing::error!("Failed to cleanup old operations: {}", e);
                }
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    async fn create_test_manager() -> BackgroundOperationManager {
        // This would create a test database manager
        // For now, we'll create a dummy one
        todo!("Implement test database manager")
    }

    #[tokio::test]
    async fn test_operation_lifecycle() {
        // Test would verify complete operation lifecycle
        // This requires the database integration to be completed first
    }

    #[tokio::test]
    async fn test_operation_cancellation() {
        // Test would verify operation cancellation
        // This requires the database integration to be completed first
    }
}