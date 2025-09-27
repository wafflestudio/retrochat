use std::sync::Arc;
use chrono::{DateTime, Utc};

use crate::database::DatabaseManager;
use crate::models::{RetrospectRequest, OperationStatus, RetrospectionAnalysisType};

#[derive(Clone)]
pub struct RetrospectRequestRepository {
    db_manager: Arc<DatabaseManager>,
}

impl RetrospectRequestRepository {
    pub fn new(db_manager: Arc<DatabaseManager>) -> Self {
        Self { db_manager }
    }

    pub async fn create(
        &self,
        request: &RetrospectRequest,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let pool = self.db_manager.pool();

        let analysis_type_str = request.analysis_type.to_string();
        let status_str = request.status.to_string();
        let started_at_str = request.started_at.to_rfc3339();
        let completed_at_str = request.completed_at.map(|dt| dt.to_rfc3339());

        sqlx::query!(
            r#"
            INSERT INTO retrospect_requests (
                id, session_id, analysis_type, status, started_at, completed_at,
                created_by, error_message, custom_prompt
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
            request.id,
            request.session_id,
            analysis_type_str,
            status_str,
            started_at_str,
            completed_at_str,
            request.created_by,
            request.error_message,
            request.custom_prompt
        )
        .execute(pool)
        .await?;

        Ok(())
    }

    pub async fn update(
        &self,
        request: &RetrospectRequest,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let pool = self.db_manager.pool();

        sqlx::query(
            r#"
            UPDATE retrospect_requests
            SET analysis_type = ?, status = ?, started_at = ?, completed_at = ?,
                created_by = ?, error_message = ?, custom_prompt = ?
            WHERE id = ?
            "#,
        )
        .bind(request.analysis_type.to_string())
        .bind(request.status.to_string())
        .bind(request.started_at.to_rfc3339())
        .bind(request.completed_at.map(|dt| dt.to_rfc3339()))
        .bind(&request.created_by)
        .bind(&request.error_message)
        .bind(&request.custom_prompt)
        .bind(&request.id)
        .execute(pool)
        .await?;

        Ok(())
    }

    pub async fn find_by_id(
        &self,
        id: &str,
    ) -> Result<Option<RetrospectRequest>, Box<dyn std::error::Error + Send + Sync>> {
        let pool = self.db_manager.pool();

        let row = sqlx::query!(
            "SELECT * FROM retrospect_requests WHERE id = ?",
            id
        )
        .fetch_optional(pool)
        .await?;

        if let Some(row) = row {
            let analysis_type = row.analysis_type.parse::<RetrospectionAnalysisType>()
                .map_err(|e| format!("Invalid analysis type '{}': {}", row.analysis_type, e))?;

            let status = row.status.parse::<OperationStatus>()
                .map_err(|e| format!("Invalid status '{}': {}", row.status, e))?;

            let started_at = DateTime::parse_from_rfc3339(&row.started_at)?
                .with_timezone(&Utc);

            let completed_at = if let Some(completed_at_str) = &row.completed_at {
                if !completed_at_str.is_empty() {
                    Some(DateTime::parse_from_rfc3339(completed_at_str)?.with_timezone(&Utc))
                } else {
                    None
                }
            } else {
                None
            };

            Ok(Some(RetrospectRequest {
                id: row.id.unwrap_or_else(|| "unknown".to_string()),
                session_id: row.session_id,
                analysis_type,
                status,
                started_at,
                completed_at,
                created_by: row.created_by,
                error_message: row.error_message,
                custom_prompt: row.custom_prompt,
            }))
        } else {
            Ok(None)
        }
    }

    pub async fn find_by_session_id(
        &self,
        session_id: &str,
    ) -> Result<Vec<RetrospectRequest>, Box<dyn std::error::Error + Send + Sync>> {
        let pool = self.db_manager.pool();

        let rows = sqlx::query!(
            "SELECT * FROM retrospect_requests WHERE session_id = ? ORDER BY started_at DESC",
            session_id
        )
        .fetch_all(pool)
        .await?;

        let mut requests = Vec::new();
        for row in rows {
            let analysis_type = row.analysis_type.parse::<RetrospectionAnalysisType>()
                .map_err(|e| format!("Invalid analysis type '{}': {}", row.analysis_type, e))?;

            let status = row.status.parse::<OperationStatus>()
                .map_err(|e| format!("Invalid status '{}': {}", row.status, e))?;

            let started_at = DateTime::parse_from_rfc3339(&row.started_at)?
                .with_timezone(&Utc);

            let completed_at = if let Some(completed_at_str) = &row.completed_at {
                if !completed_at_str.is_empty() {
                    Some(DateTime::parse_from_rfc3339(completed_at_str)?.with_timezone(&Utc))
                } else {
                    None
                }
            } else {
                None
            };

            requests.push(RetrospectRequest {
                id: row.id.unwrap_or_else(|| "unknown".to_string()),
                session_id: row.session_id,
                analysis_type,
                status,
                started_at,
                completed_at,
                created_by: row.created_by,
                error_message: row.error_message,
                custom_prompt: row.custom_prompt,
            });
        }

        Ok(requests)
    }

    pub async fn find_active_requests(
        &self,
    ) -> Result<Vec<RetrospectRequest>, Box<dyn std::error::Error + Send + Sync>> {
        let pool = self.db_manager.pool();

        let rows = sqlx::query!(
            "SELECT * FROM retrospect_requests WHERE status IN ('pending', 'running') ORDER BY started_at ASC"
        )
        .fetch_all(pool)
        .await?;

        let mut requests = Vec::new();
        for row in rows {
            let analysis_type = row.analysis_type.parse::<RetrospectionAnalysisType>()
                .map_err(|e| format!("Invalid analysis type '{}': {}", row.analysis_type, e))?;

            let status = row.status.parse::<OperationStatus>()
                .map_err(|e| format!("Invalid status '{}': {}", row.status, e))?;

            let started_at = DateTime::parse_from_rfc3339(&row.started_at)?
                .with_timezone(&Utc);

            let completed_at = if let Some(completed_at_str) = &row.completed_at {
                if !completed_at_str.is_empty() {
                    Some(DateTime::parse_from_rfc3339(completed_at_str)?.with_timezone(&Utc))
                } else {
                    None
                }
            } else {
                None
            };

            requests.push(RetrospectRequest {
                id: row.id.unwrap_or_else(|| "unknown".to_string()),
                session_id: row.session_id,
                analysis_type,
                status,
                started_at,
                completed_at,
                created_by: row.created_by,
                error_message: row.error_message,
                custom_prompt: row.custom_prompt,
            });
        }

        Ok(requests)
    }

    pub async fn find_by_status(
        &self,
        status: OperationStatus,
    ) -> Result<Vec<RetrospectRequest>, Box<dyn std::error::Error + Send + Sync>> {
        let pool = self.db_manager.pool();

        let status_str = status.to_string();
        let rows = sqlx::query!(
            "SELECT * FROM retrospect_requests WHERE status = ? ORDER BY started_at DESC",
            status_str
        )
        .fetch_all(pool)
        .await?;

        let mut requests = Vec::new();
        for row in rows {
            let analysis_type = row.analysis_type.parse::<RetrospectionAnalysisType>()
                .map_err(|e| format!("Invalid analysis type '{}': {}", row.analysis_type, e))?;

            let status = row.status.parse::<OperationStatus>()
                .map_err(|e| format!("Invalid status '{}': {}", row.status, e))?;

            let started_at = DateTime::parse_from_rfc3339(&row.started_at)?
                .with_timezone(&Utc);

            let completed_at = if let Some(completed_at_str) = &row.completed_at {
                if !completed_at_str.is_empty() {
                    Some(DateTime::parse_from_rfc3339(completed_at_str)?.with_timezone(&Utc))
                } else {
                    None
                }
            } else {
                None
            };

            requests.push(RetrospectRequest {
                id: row.id.unwrap_or_else(|| "unknown".to_string()),
                session_id: row.session_id,
                analysis_type,
                status,
                started_at,
                completed_at,
                created_by: row.created_by,
                error_message: row.error_message,
                custom_prompt: row.custom_prompt,
            });
        }

        Ok(requests)
    }

    pub async fn find_by_created_by(
        &self,
        created_by: &str,
    ) -> Result<Vec<RetrospectRequest>, Box<dyn std::error::Error + Send + Sync>> {
        let pool = self.db_manager.pool();

        let rows = sqlx::query!(
            "SELECT * FROM retrospect_requests WHERE created_by = ? ORDER BY started_at DESC",
            created_by
        )
        .fetch_all(pool)
        .await?;

        let mut requests = Vec::new();
        for row in rows {
            let analysis_type = row.analysis_type.parse::<RetrospectionAnalysisType>()
                .map_err(|e| format!("Invalid analysis type '{}': {}", row.analysis_type, e))?;

            let status = row.status.parse::<OperationStatus>()
                .map_err(|e| format!("Invalid status '{}': {}", row.status, e))?;

            let started_at = DateTime::parse_from_rfc3339(&row.started_at)?
                .with_timezone(&Utc);

            let completed_at = if let Some(completed_at_str) = &row.completed_at {
                if !completed_at_str.is_empty() {
                    Some(DateTime::parse_from_rfc3339(completed_at_str)?.with_timezone(&Utc))
                } else {
                    None
                }
            } else {
                None
            };

            requests.push(RetrospectRequest {
                id: row.id.unwrap_or_else(|| "unknown".to_string()),
                session_id: row.session_id,
                analysis_type,
                status,
                started_at,
                completed_at,
                created_by: row.created_by,
                error_message: row.error_message,
                custom_prompt: row.custom_prompt,
            });
        }

        Ok(requests)
    }

    pub async fn find_recent(
        &self,
        limit: Option<usize>,
    ) -> Result<Vec<RetrospectRequest>, Box<dyn std::error::Error + Send + Sync>> {
        let pool = self.db_manager.pool();

        let limit = limit.unwrap_or(10) as i64;

        let rows = sqlx::query!(
            "SELECT * FROM retrospect_requests ORDER BY started_at DESC LIMIT ?",
            limit
        )
        .fetch_all(pool)
        .await?;

        let mut requests = Vec::new();
        for row in rows {
            let analysis_type = row.analysis_type.parse::<RetrospectionAnalysisType>()
                .map_err(|e| format!("Invalid analysis type '{}': {}", row.analysis_type, e))?;

            let status = row.status.parse::<OperationStatus>()
                .map_err(|e| format!("Invalid status '{}': {}", row.status, e))?;

            let started_at = DateTime::parse_from_rfc3339(&row.started_at)?
                .with_timezone(&Utc);

            let completed_at = if let Some(completed_at_str) = &row.completed_at {
                if !completed_at_str.is_empty() {
                    Some(DateTime::parse_from_rfc3339(completed_at_str)?.with_timezone(&Utc))
                } else {
                    None
                }
            } else {
                None
            };

            requests.push(RetrospectRequest {
                id: row.id.unwrap_or_else(|| "unknown".to_string()),
                session_id: row.session_id,
                analysis_type,
                status,
                started_at,
                completed_at,
                created_by: row.created_by,
                error_message: row.error_message,
                custom_prompt: row.custom_prompt,
            });
        }

        Ok(requests)
    }

    pub async fn find_since(
        &self,
        since: DateTime<Utc>,
    ) -> Result<Vec<RetrospectRequest>, Box<dyn std::error::Error + Send + Sync>> {
        let pool = self.db_manager.pool();

        let since_str = since.to_rfc3339();
        let rows = sqlx::query!(
            "SELECT * FROM retrospect_requests WHERE started_at >= ? ORDER BY started_at DESC",
            since_str
        )
        .fetch_all(pool)
        .await?;

        let mut requests = Vec::new();
        for row in rows {
            let analysis_type = row.analysis_type.parse::<RetrospectionAnalysisType>()
                .map_err(|e| format!("Invalid analysis type '{}': {}", row.analysis_type, e))?;

            let status = row.status.parse::<OperationStatus>()
                .map_err(|e| format!("Invalid status '{}': {}", row.status, e))?;

            let started_at = DateTime::parse_from_rfc3339(&row.started_at)?
                .with_timezone(&Utc);

            let completed_at = if let Some(completed_at_str) = &row.completed_at {
                if !completed_at_str.is_empty() {
                    Some(DateTime::parse_from_rfc3339(completed_at_str)?.with_timezone(&Utc))
                } else {
                    None
                }
            } else {
                None
            };

            requests.push(RetrospectRequest {
                id: row.id.unwrap_or_else(|| "unknown".to_string()),
                session_id: row.session_id,
                analysis_type,
                status,
                started_at,
                completed_at,
                created_by: row.created_by,
                error_message: row.error_message,
                custom_prompt: row.custom_prompt,
            });
        }

        Ok(requests)
    }

    pub async fn delete_by_id(
        &self,
        id: &str,
    ) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        let pool = self.db_manager.pool();

        let result = sqlx::query!(
            "DELETE FROM retrospect_requests WHERE id = ?",
            id
        )
        .execute(pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    pub async fn delete_completed_before(
        &self,
        before: DateTime<Utc>,
    ) -> Result<u64, Box<dyn std::error::Error + Send + Sync>> {
        let pool = self.db_manager.pool();

        let before_str = before.to_rfc3339();
        let result = sqlx::query!(
            "DELETE FROM retrospect_requests WHERE completed_at IS NOT NULL AND completed_at < ?",
            before_str
        )
        .execute(pool)
        .await?;

        Ok(result.rows_affected())
    }

    pub async fn count_by_status(
        &self,
        status: OperationStatus,
    ) -> Result<u64, Box<dyn std::error::Error + Send + Sync>> {
        let pool = self.db_manager.pool();

        let status_str = status.to_string();
        let row = sqlx::query!(
            "SELECT COUNT(*) as count FROM retrospect_requests WHERE status = ?",
            status_str
        )
        .fetch_one(pool)
        .await?;

        Ok(row.count as u64)
    }

    pub async fn count_active(&self) -> Result<u64, Box<dyn std::error::Error + Send + Sync>> {
        let pool = self.db_manager.pool();

        let row = sqlx::query!(
            "SELECT COUNT(*) as count FROM retrospect_requests WHERE status IN ('pending', 'running')"
        )
        .fetch_one(pool)
        .await?;

        Ok(row.count.unwrap_or(0) as u64)
    }

}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::Database;

    #[tokio::test]
    async fn test_create_and_find_request() {
        let database = Database::new_in_memory().await.unwrap();
        database.initialize().await.unwrap();

        let repo = RetrospectRequestRepository::new(Arc::new(database.manager));

        let request = RetrospectRequest::new(
            "session-123".to_string(),
            RetrospectionAnalysisType::UserInteractionAnalysis,
            Some("test_user".to_string()),
            None,
        );

        repo.create(&request).await.unwrap();

        let found = repo.find_by_id(&request.id).await.unwrap();
        assert!(found.is_some());

        let found_request = found.unwrap();
        assert_eq!(found_request.session_id, request.session_id);
        assert_eq!(found_request.analysis_type, request.analysis_type);
        assert_eq!(found_request.status, request.status);
    }

    #[tokio::test]
    async fn test_update_request() {
        let database = Database::new_in_memory().await.unwrap();
        database.initialize().await.unwrap();

        let repo = RetrospectRequestRepository::new(Arc::new(database.manager));

        let mut request = RetrospectRequest::new(
            "session-456".to_string(),
            RetrospectionAnalysisType::CollaborationInsights,
            Some("test_user".to_string()),
            None,
        );

        repo.create(&request).await.unwrap();

        request.mark_completed();
        repo.update(&request).await.unwrap();

        let found = repo.find_by_id(&request.id).await.unwrap().unwrap();
        assert_eq!(found.status, OperationStatus::Completed);
        assert!(found.completed_at.is_some());
    }

    #[tokio::test]
    async fn test_find_by_session_id() {
        let database = Database::new_in_memory().await.unwrap();
        database.initialize().await.unwrap();

        let repo = RetrospectRequestRepository::new(Arc::new(database.manager));

        let session_id = "session-789".to_string();

        let request1 = RetrospectRequest::new(
            session_id.clone(),
            RetrospectionAnalysisType::QuestionQuality,
            Some("test_user".to_string()),
            None,
        );

        let request2 = RetrospectRequest::new(
            session_id.clone(),
            RetrospectionAnalysisType::TaskBreakdown,
            Some("test_user".to_string()),
            None,
        );

        repo.create(&request1).await.unwrap();
        repo.create(&request2).await.unwrap();

        let found = repo.find_by_session_id(&session_id).await.unwrap();
        assert_eq!(found.len(), 2);
    }
}