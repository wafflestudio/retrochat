use chrono::{DateTime, Utc};
use std::sync::Arc;

use crate::database::DatabaseManager;
use crate::models::Retrospection;

#[derive(Clone)]
pub struct RetrospectionRepository {
    db_manager: Arc<DatabaseManager>,
}

impl RetrospectionRepository {
    pub fn new(db_manager: Arc<DatabaseManager>) -> Self {
        Self { db_manager }
    }

    pub async fn create(
        &self,
        retrospection: &Retrospection,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let pool = self.db_manager.pool();

        let created_at_str = retrospection.created_at.to_rfc3339();
        let response_text = format!(
            "Insights: {}\n\nReflection: {}\n\nRecommendations: {}",
            retrospection.insights, retrospection.reflection, retrospection.recommendations
        );
        let response_time_ms = retrospection.response_time.map(|d| d.as_millis() as i32);

        sqlx::query!(
            r#"
            INSERT INTO retrospections (
                id, retrospect_request_id, response_text, token_usage,
                response_time_ms, model_used, metadata, created_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            "#,
            retrospection.id,
            retrospection.request_id,
            response_text,
            retrospection.token_usage,
            response_time_ms,
            Some("gemini-pro"),
            retrospection.metadata,
            created_at_str
        )
        .execute(pool)
        .await?;

        Ok(())
    }

    pub async fn find_by_id(
        &self,
        id: &str,
    ) -> Result<Option<Retrospection>, Box<dyn std::error::Error + Send + Sync>> {
        let pool = self.db_manager.pool();

        let row = sqlx::query!("SELECT * FROM retrospections WHERE id = ?", id)
            .fetch_optional(pool)
            .await?;

        if let Some(row) = row {
            let created_at = DateTime::parse_from_rfc3339(&row.created_at)?.with_timezone(&Utc);

            // Parse the combined response_text back into separate fields
            let (insights, reflection, recommendations) =
                self.parse_response_text(&row.response_text);

            Ok(Some(Retrospection {
                id: row.id.unwrap_or_else(|| "unknown".to_string()),
                request_id: row.retrospect_request_id,
                insights,
                reflection,
                recommendations,
                metadata: row.metadata,
                created_at,
                token_usage: row.token_usage.map(|t| t as u32),
                response_time: row
                    .response_time_ms
                    .map(|ms| std::time::Duration::from_millis(ms as u64)),
            }))
        } else {
            Ok(None)
        }
    }

    pub async fn find_by_request_id(
        &self,
        request_id: &str,
    ) -> Result<Vec<Retrospection>, Box<dyn std::error::Error + Send + Sync>> {
        let pool = self.db_manager.pool();

        let rows = sqlx::query!(
            "SELECT * FROM retrospections WHERE retrospect_request_id = ? ORDER BY created_at DESC",
            request_id
        )
        .fetch_all(pool)
        .await?;

        let mut retrospections = Vec::new();
        for row in rows {
            let created_at = DateTime::parse_from_rfc3339(&row.created_at)?.with_timezone(&Utc);

            let (insights, reflection, recommendations) =
                self.parse_response_text(&row.response_text);

            retrospections.push(Retrospection {
                id: row.id.unwrap_or_else(|| "unknown".to_string()),
                request_id: row.retrospect_request_id,
                insights,
                reflection,
                recommendations,
                metadata: row.metadata,
                created_at,
                token_usage: row.token_usage.map(|t| t as u32),
                response_time: row
                    .response_time_ms
                    .map(|ms| std::time::Duration::from_millis(ms as u64)),
            });
        }

        Ok(retrospections)
    }

    pub async fn find_recent(
        &self,
        limit: Option<usize>,
    ) -> Result<Vec<Retrospection>, Box<dyn std::error::Error + Send + Sync>> {
        let pool = self.db_manager.pool();

        let limit = limit.unwrap_or(10) as i64;

        let rows = sqlx::query!(
            "SELECT * FROM retrospections ORDER BY created_at DESC LIMIT ?",
            limit
        )
        .fetch_all(pool)
        .await?;

        let mut retrospections = Vec::new();
        for row in rows {
            let created_at = DateTime::parse_from_rfc3339(&row.created_at)?.with_timezone(&Utc);

            let (insights, reflection, recommendations) =
                self.parse_response_text(&row.response_text);

            retrospections.push(Retrospection {
                id: row.id.unwrap_or_else(|| "unknown".to_string()),
                request_id: row.retrospect_request_id,
                insights,
                reflection,
                recommendations,
                metadata: row.metadata,
                created_at,
                token_usage: row.token_usage.map(|t| t as u32),
                response_time: row
                    .response_time_ms
                    .map(|ms| std::time::Duration::from_millis(ms as u64)),
            });
        }

        Ok(retrospections)
    }

    pub async fn find_since(
        &self,
        since: DateTime<Utc>,
    ) -> Result<Vec<Retrospection>, Box<dyn std::error::Error + Send + Sync>> {
        let pool = self.db_manager.pool();

        let since_str = since.to_rfc3339();
        let rows = sqlx::query!(
            "SELECT * FROM retrospections WHERE created_at >= ? ORDER BY created_at DESC",
            since_str
        )
        .fetch_all(pool)
        .await?;

        let mut retrospections = Vec::new();
        for row in rows {
            let created_at = DateTime::parse_from_rfc3339(&row.created_at)?.with_timezone(&Utc);

            let (insights, reflection, recommendations) =
                self.parse_response_text(&row.response_text);

            retrospections.push(Retrospection {
                id: row.id.unwrap_or_else(|| "unknown".to_string()),
                request_id: row.retrospect_request_id,
                insights,
                reflection,
                recommendations,
                metadata: row.metadata,
                created_at,
                token_usage: row.token_usage.map(|t| t as u32),
                response_time: row
                    .response_time_ms
                    .map(|ms| std::time::Duration::from_millis(ms as u64)),
            });
        }

        Ok(retrospections)
    }

    pub async fn delete_by_id(
        &self,
        id: &str,
    ) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        let pool = self.db_manager.pool();

        let result = sqlx::query!("DELETE FROM retrospections WHERE id = ?", id)
            .execute(pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }

    pub async fn delete_by_request_id(
        &self,
        request_id: &str,
    ) -> Result<u64, Box<dyn std::error::Error + Send + Sync>> {
        let pool = self.db_manager.pool();

        let result = sqlx::query!(
            "DELETE FROM retrospections WHERE retrospect_request_id = ?",
            request_id
        )
        .execute(pool)
        .await?;

        Ok(result.rows_affected())
    }

    pub async fn delete_before(
        &self,
        before: DateTime<Utc>,
    ) -> Result<u64, Box<dyn std::error::Error + Send + Sync>> {
        let pool = self.db_manager.pool();

        let before_str = before.to_rfc3339();
        let result = sqlx::query!(
            "DELETE FROM retrospections WHERE created_at < ?",
            before_str
        )
        .execute(pool)
        .await?;

        Ok(result.rows_affected())
    }

    pub async fn count(&self) -> Result<u64, Box<dyn std::error::Error + Send + Sync>> {
        let pool = self.db_manager.pool();

        let row = sqlx::query!("SELECT COUNT(*) as count FROM retrospections")
            .fetch_one(pool)
            .await?;

        Ok(row.count as u64)
    }

    pub async fn count_by_request_id(
        &self,
        request_id: &str,
    ) -> Result<u64, Box<dyn std::error::Error + Send + Sync>> {
        let pool = self.db_manager.pool();

        let row = sqlx::query!(
            "SELECT COUNT(*) as count FROM retrospections WHERE retrospect_request_id = ?",
            request_id
        )
        .fetch_one(pool)
        .await?;

        Ok(row.count as u64)
    }

    pub async fn get_by_session_id(
        &self,
        session_id: &str,
    ) -> Result<Vec<Retrospection>, Box<dyn std::error::Error + Send + Sync>> {
        let pool = self.db_manager.pool();

        let rows = sqlx::query!(
            r#"
            SELECT r.* FROM retrospections r
            JOIN retrospect_requests rr ON r.retrospect_request_id = rr.id
            WHERE rr.session_id = ?
            ORDER BY r.created_at DESC
            "#,
            session_id
        )
        .fetch_all(pool)
        .await?;

        let mut retrospections = Vec::new();
        for row in rows {
            let created_at = DateTime::parse_from_rfc3339(&row.created_at)?.with_timezone(&Utc);

            let (insights, reflection, recommendations) =
                self.parse_response_text(&row.response_text);

            retrospections.push(Retrospection {
                id: row.id.unwrap_or_else(|| "unknown".to_string()),
                request_id: row.retrospect_request_id,
                insights,
                reflection,
                recommendations,
                metadata: row.metadata,
                created_at,
                token_usage: row.token_usage.map(|t| t as u32),
                response_time: row
                    .response_time_ms
                    .map(|ms| std::time::Duration::from_millis(ms as u64)),
            });
        }

        Ok(retrospections)
    }

    fn parse_response_text(&self, response_text: &str) -> (String, String, String) {
        // Default values in case parsing fails
        let mut insights = "".to_string();
        let mut reflection = "".to_string();
        let mut recommendations = "".to_string();

        // Simple parsing of the formatted response text
        let sections: Vec<&str> = response_text.split("\n\n").collect();

        for section in sections {
            if section.starts_with("Insights: ") {
                insights = section.strip_prefix("Insights: ").unwrap_or("").to_string();
            } else if section.starts_with("Reflection: ") {
                reflection = section
                    .strip_prefix("Reflection: ")
                    .unwrap_or("")
                    .to_string();
            } else if section.starts_with("Recommendations: ") {
                recommendations = section
                    .strip_prefix("Recommendations: ")
                    .unwrap_or("")
                    .to_string();
            }
        }

        (insights, reflection, recommendations)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::{ChatSessionRepository, Database, RetrospectRequestRepository};
    use crate::models::{ChatSession, Provider, RetrospectRequest};
    use std::sync::Arc;

    #[tokio::test]
    async fn test_create_and_find_retrospection() {
        let database = Database::new_in_memory().await.unwrap();
        database.initialize().await.unwrap();

        let db_manager = Arc::new(database.manager);

        // Create a chat session first (required for foreign key constraint)
        let session_repo = ChatSessionRepository::new(&db_manager);
        let session = ChatSession::new(
            Provider::ClaudeCode,
            "/test/path".to_string(),
            "test-hash".to_string(),
            Utc::now(),
        );
        session_repo.create(&session).await.unwrap();

        // Create a retrospect request (required for foreign key constraint)
        let request_repo = RetrospectRequestRepository::new(db_manager.clone());
        let request =
            RetrospectRequest::new(session.id.to_string(), Some("test_user".to_string()), None);
        request_repo.create(&request).await.unwrap();

        let repo = RetrospectionRepository::new(db_manager);

        let retrospection = Retrospection::new(
            request.id.clone(),
            "Some insights".to_string(),
            "Some reflection".to_string(),
            "Some recommendations".to_string(),
            None,
        );

        repo.create(&retrospection).await.unwrap();

        let found = repo.find_by_id(&retrospection.id).await.unwrap();
        assert!(found.is_some());

        let found_retrospection = found.unwrap();
        assert_eq!(found_retrospection.request_id, retrospection.request_id);
        assert_eq!(found_retrospection.insights, retrospection.insights);
        assert_eq!(found_retrospection.reflection, retrospection.reflection);
        assert_eq!(
            found_retrospection.recommendations,
            retrospection.recommendations
        );
    }

    #[tokio::test]
    async fn test_find_by_request_id() {
        let database = Database::new_in_memory().await.unwrap();
        database.initialize().await.unwrap();

        let db_manager = Arc::new(database.manager);

        // Create a chat session first (required for foreign key constraint)
        let session_repo = ChatSessionRepository::new(&db_manager);
        let session = ChatSession::new(
            Provider::ClaudeCode,
            "/test/path".to_string(),
            "test-hash".to_string(),
            Utc::now(),
        );
        session_repo.create(&session).await.unwrap();

        // Create a retrospect request (required for foreign key constraint)
        let request_repo = RetrospectRequestRepository::new(db_manager.clone());
        let request =
            RetrospectRequest::new(session.id.to_string(), Some("test_user".to_string()), None);
        request_repo.create(&request).await.unwrap();

        let repo = RetrospectionRepository::new(db_manager);

        let request_id = request.id.clone();

        let retrospection1 = Retrospection::new(
            request_id.clone(),
            "Insights 1".to_string(),
            "Reflection 1".to_string(),
            "Recommendations 1".to_string(),
            None,
        );

        let retrospection2 = Retrospection::new(
            request_id.clone(),
            "Insights 2".to_string(),
            "Reflection 2".to_string(),
            "Recommendations 2".to_string(),
            None,
        );

        repo.create(&retrospection1).await.unwrap();
        repo.create(&retrospection2).await.unwrap();

        let found = repo.find_by_request_id(&request_id).await.unwrap();
        assert_eq!(found.len(), 2);
    }
}
