use crate::models::{AnalysisRequest, RequestStatus};
use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, Row};
use serde::{Deserialize, Serialize};
use serde_json;
use std::collections::HashMap;
use uuid::Uuid;

pub struct AnalysisRequestRepository<'a> {
    conn: &'a Connection,
}

impl<'a> AnalysisRequestRepository<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    pub fn create(&self, request: &AnalysisRequest) -> Result<()> {
        let template_variables_json = serde_json::to_string(&request.template_variables)
            .map_err(|e| anyhow!("Failed to serialize template variables: {}", e))?;

        self.conn.execute(
            r#"
            INSERT INTO analysis_requests (
                id, session_id, prompt_template_id, template_variables,
                status, error_message, created_at, started_at, completed_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
            "#,
            params![
                request.id.to_string(),
                request.session_id.to_string(),
                request.prompt_template_id,
                template_variables_json,
                request.status.to_string(),
                request.error_message,
                request.created_at.to_rfc3339(),
                request.started_at.map(|dt| dt.to_rfc3339()),
                request.completed_at.map(|dt| dt.to_rfc3339()),
            ],
        )?;

        Ok(())
    }

    pub fn find_by_id(&self, id: &Uuid) -> Result<Option<AnalysisRequest>> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT id, session_id, prompt_template_id, template_variables,
                   status, error_message, created_at, started_at, completed_at
            FROM analysis_requests
            WHERE id = ?1
            "#,
        )?;

        let mut rows = stmt.query_map(params![id.to_string()], |row| self.row_to_request(row))?;

        match rows.next() {
            Some(request) => Ok(Some(request?)),
            None => Ok(None),
        }
    }

    pub fn find_by_session_id(&self, session_id: &Uuid) -> Result<Vec<AnalysisRequest>> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT id, session_id, prompt_template_id, template_variables,
                   status, error_message, created_at, started_at, completed_at
            FROM analysis_requests
            WHERE session_id = ?1
            ORDER BY created_at DESC
            "#,
        )?;

        let request_iter = stmt.query_map(params![session_id.to_string()], |row| {
            self.row_to_request(row)
        })?;

        let mut requests = Vec::new();
        for request in request_iter {
            requests.push(request?);
        }

        Ok(requests)
    }

    pub fn find_by_status(&self, status: &RequestStatus) -> Result<Vec<AnalysisRequest>> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT id, session_id, prompt_template_id, template_variables,
                   status, error_message, created_at, started_at, completed_at
            FROM analysis_requests
            WHERE status = ?1
            ORDER BY created_at ASC
            "#,
        )?;

        let request_iter =
            stmt.query_map(params![status.to_string()], |row| self.row_to_request(row))?;

        let mut requests = Vec::new();
        for request in request_iter {
            requests.push(request?);
        }

        Ok(requests)
    }

    pub fn get_queue(&self) -> Result<Vec<AnalysisRequest>> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT id, session_id, prompt_template_id, template_variables,
                   status, error_message, created_at, started_at, completed_at,
                   (julianday('now') - julianday(created_at)) * 24 * 60 as age_minutes
            FROM analysis_requests
            WHERE status = 'queued'
            ORDER BY
                CASE WHEN error_message IS NOT NULL THEN 0 ELSE 1 END,  -- Failed requests first
                age_minutes DESC  -- Older requests first within each group
            "#,
        )?;

        let request_iter = stmt.query_map([], |row| self.row_to_request(row))?;

        let mut requests = Vec::new();
        for request in request_iter {
            requests.push(request?);
        }

        Ok(requests)
    }

    pub fn get_next_queued_request(&self) -> Result<Option<AnalysisRequest>> {
        let queue = self.get_queue()?;
        Ok(queue.into_iter().next())
    }

    pub fn find_processing_requests(&self) -> Result<Vec<AnalysisRequest>> {
        self.find_by_status(&RequestStatus::Processing)
    }

    pub fn find_stale_requests(&self, max_age_hours: i64) -> Result<Vec<AnalysisRequest>> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT id, session_id, prompt_template_id, template_variables,
                   status, error_message, created_at, started_at, completed_at
            FROM analysis_requests
            WHERE
                (status = 'queued' AND (julianday('now') - julianday(created_at)) * 24 > ?1)
                OR
                (status = 'processing' AND started_at IS NOT NULL AND (julianday('now') - julianday(started_at)) * 24 > 1)
            ORDER BY created_at ASC
            "#,
        )?;

        let request_iter =
            stmt.query_map(params![max_age_hours], |row| self.row_to_request(row))?;

        let mut requests = Vec::new();
        for request in request_iter {
            requests.push(request?);
        }

        Ok(requests)
    }

    pub fn update(&self, request: &AnalysisRequest) -> Result<()> {
        let template_variables_json = serde_json::to_string(&request.template_variables)
            .map_err(|e| anyhow!("Failed to serialize template variables: {}", e))?;

        let rows_affected = self.conn.execute(
            r#"
            UPDATE analysis_requests
            SET session_id = ?2, prompt_template_id = ?3, template_variables = ?4,
                status = ?5, error_message = ?6, started_at = ?7, completed_at = ?8
            WHERE id = ?1
            "#,
            params![
                request.id.to_string(),
                request.session_id.to_string(),
                request.prompt_template_id,
                template_variables_json,
                request.status.to_string(),
                request.error_message,
                request.started_at.map(|dt| dt.to_rfc3339()),
                request.completed_at.map(|dt| dt.to_rfc3339()),
            ],
        )?;

        if rows_affected == 0 {
            return Err(anyhow!("Request with id {} not found", request.id));
        }

        Ok(())
    }

    pub fn delete(&self, id: &Uuid) -> Result<bool> {
        let rows_affected = self.conn.execute(
            "DELETE FROM analysis_requests WHERE id = ?1",
            params![id.to_string()],
        )?;

        Ok(rows_affected > 0)
    }

    pub fn cleanup_completed_requests(&self, older_than_days: u32) -> Result<u32> {
        let rows_affected = self.conn.execute(
            r#"
            DELETE FROM analysis_requests
            WHERE status IN ('completed', 'failed')
            AND (julianday('now') - julianday(completed_at)) > ?1
            "#,
            params![older_than_days],
        )?;

        Ok(rows_affected as u32)
    }

    pub fn count_by_status(&self, status: &RequestStatus) -> Result<u32> {
        let mut stmt = self
            .conn
            .prepare("SELECT COUNT(*) FROM analysis_requests WHERE status = ?1")?;

        let count: i64 = stmt.query_row(params![status.to_string()], |row| row.get(0))?;

        Ok(count as u32)
    }

    pub fn count_by_session(&self, session_id: &Uuid) -> Result<u32> {
        let mut stmt = self
            .conn
            .prepare("SELECT COUNT(*) FROM analysis_requests WHERE session_id = ?1")?;

        let count: i64 = stmt.query_row(params![session_id.to_string()], |row| row.get(0))?;

        Ok(count as u32)
    }

    pub fn get_queue_statistics(&self) -> Result<QueueStatistics> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT
                status,
                COUNT(*) as count,
                AVG((julianday('now') - julianday(created_at)) * 24 * 60) as avg_age_minutes,
                MAX((julianday('now') - julianday(created_at)) * 24 * 60) as max_age_minutes
            FROM analysis_requests
            WHERE status IN ('queued', 'processing')
            GROUP BY status
            "#,
        )?;

        let mut queued_count = 0u32;
        let mut processing_count = 0u32;
        let mut avg_queue_age_minutes = 0.0;
        let mut max_queue_age_minutes = 0.0;

        let stat_iter = stmt.query_map([], |row| {
            let status: String = row.get(0)?;
            let count: i64 = row.get(1)?;
            let avg_age: Option<f64> = row.get(2)?;
            let max_age: Option<f64> = row.get(3)?;

            Ok((
                status,
                count as u32,
                avg_age.unwrap_or(0.0),
                max_age.unwrap_or(0.0),
            ))
        })?;

        for stat_result in stat_iter {
            let (status, count, avg_age, max_age) = stat_result?;
            match status.as_str() {
                "queued" => {
                    queued_count = count;
                    avg_queue_age_minutes = avg_age;
                    max_queue_age_minutes = max_age;
                }
                "processing" => {
                    processing_count = count;
                }
                _ => {}
            }
        }

        // Get failed request count that can be retried
        let failed_retryable_count: i64 = self
            .conn
            .query_row(
                "SELECT COUNT(*) FROM analysis_requests WHERE status = 'failed'",
                [],
                |row| row.get(0),
            )
            .unwrap_or(0);

        Ok(QueueStatistics {
            queued_count,
            processing_count,
            failed_retryable_count: failed_retryable_count as u32,
            avg_queue_age_minutes,
            max_queue_age_minutes,
        })
    }

    pub fn find_duplicate_requests(
        &self,
        session_id: &Uuid,
        template_id: &str,
    ) -> Result<Vec<AnalysisRequest>> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT id, session_id, prompt_template_id, template_variables,
                   status, error_message, created_at, started_at, completed_at
            FROM analysis_requests
            WHERE session_id = ?1 AND prompt_template_id = ?2
            ORDER BY created_at DESC
            "#,
        )?;

        let request_iter = stmt.query_map(params![session_id.to_string(), template_id], |row| {
            self.row_to_request(row)
        })?;

        let mut requests = Vec::new();
        for request in request_iter {
            requests.push(request?);
        }

        Ok(requests)
    }

    pub fn retry_failed_request(&self, id: &Uuid) -> Result<bool> {
        let rows_affected = self.conn.execute(
            r#"
            UPDATE analysis_requests
            SET status = 'queued', error_message = NULL, started_at = NULL, completed_at = NULL
            WHERE id = ?1 AND status = 'failed'
            "#,
            params![id.to_string()],
        )?;

        Ok(rows_affected > 0)
    }

    pub fn mark_processing(&self, id: &Uuid) -> Result<bool> {
        let now = Utc::now();
        let rows_affected = self.conn.execute(
            r#"
            UPDATE analysis_requests
            SET status = 'processing', started_at = ?2, error_message = NULL
            WHERE id = ?1 AND status = 'queued'
            "#,
            params![id.to_string(), now.to_rfc3339()],
        )?;

        Ok(rows_affected > 0)
    }

    pub fn mark_completed(&self, id: &Uuid) -> Result<bool> {
        let now = Utc::now();
        let rows_affected = self.conn.execute(
            r#"
            UPDATE analysis_requests
            SET status = 'completed', completed_at = ?2, error_message = NULL
            WHERE id = ?1 AND status = 'processing'
            "#,
            params![id.to_string(), now.to_rfc3339()],
        )?;

        Ok(rows_affected > 0)
    }

    pub fn mark_failed(&self, id: &Uuid, error_message: &str) -> Result<bool> {
        let now = Utc::now();
        let rows_affected = self.conn.execute(
            r#"
            UPDATE analysis_requests
            SET status = 'failed', completed_at = ?2, error_message = ?3
            WHERE id = ?1 AND status = 'processing'
            "#,
            params![id.to_string(), now.to_rfc3339(), error_message],
        )?;

        Ok(rows_affected > 0)
    }

    fn row_to_request(&self, row: &Row) -> rusqlite::Result<AnalysisRequest> {
        let id_str: String = row.get(0)?;
        let session_id_str: String = row.get(1)?;
        let template_variables_json: String = row.get(3)?;
        let status_str: String = row.get(4)?;
        let created_at_str: String = row.get(6)?;
        let started_at_str: Option<String> = row.get(7)?;
        let completed_at_str: Option<String> = row.get(8)?;

        let id = Uuid::parse_str(&id_str).map_err(|_e| {
            rusqlite::Error::InvalidColumnType(0, "UUID".to_string(), rusqlite::types::Type::Text)
        })?;
        let session_id = Uuid::parse_str(&session_id_str).map_err(|_e| {
            rusqlite::Error::InvalidColumnType(1, "UUID".to_string(), rusqlite::types::Type::Text)
        })?;

        let template_variables: HashMap<String, String> =
            serde_json::from_str(&template_variables_json).map_err(|_e| {
                rusqlite::Error::InvalidColumnType(
                    3,
                    "JSON".to_string(),
                    rusqlite::types::Type::Text,
                )
            })?;

        let status: RequestStatus = status_str.parse().map_err(|_e| {
            rusqlite::Error::InvalidColumnType(
                4,
                "RequestStatus".to_string(),
                rusqlite::types::Type::Text,
            )
        })?;

        let created_at = DateTime::parse_from_rfc3339(&created_at_str)
            .map_err(|_e| {
                rusqlite::Error::InvalidColumnType(
                    6,
                    "DateTime".to_string(),
                    rusqlite::types::Type::Text,
                )
            })?
            .with_timezone(&Utc);

        let started_at = if let Some(started_str) = started_at_str {
            Some(
                DateTime::parse_from_rfc3339(&started_str)
                    .map_err(|_e| {
                        rusqlite::Error::InvalidColumnType(
                            7,
                            "DateTime".to_string(),
                            rusqlite::types::Type::Text,
                        )
                    })?
                    .with_timezone(&Utc),
            )
        } else {
            None
        };

        let completed_at = if let Some(completed_str) = completed_at_str {
            Some(
                DateTime::parse_from_rfc3339(&completed_str)
                    .map_err(|_e| {
                        rusqlite::Error::InvalidColumnType(
                            8,
                            "DateTime".to_string(),
                            rusqlite::types::Type::Text,
                        )
                    })?
                    .with_timezone(&Utc),
            )
        } else {
            None
        };

        Ok(AnalysisRequest {
            id,
            session_id,
            prompt_template_id: row.get(2)?,
            template_variables,
            status,
            error_message: row.get(5)?,
            created_at,
            started_at,
            completed_at,
        })
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct QueueStatistics {
    pub queued_count: u32,
    pub processing_count: u32,
    pub failed_retryable_count: u32,
    pub avg_queue_age_minutes: f64,
    pub max_queue_age_minutes: f64,
}

impl QueueStatistics {
    pub fn total_pending(&self) -> u32 {
        self.queued_count + self.processing_count
    }

    pub fn is_backlogged(&self, threshold_minutes: f64) -> bool {
        self.avg_queue_age_minutes > threshold_minutes
    }

    pub fn get_summary(&self) -> String {
        format!(
            "Queue: {} queued, {} processing, {} failed (avg age: {:.1}min)",
            self.queued_count,
            self.processing_count,
            self.failed_retryable_count,
            self.avg_queue_age_minutes
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::AnalysisRequest;
    use rusqlite::Connection;
    use std::collections::HashMap;
    use tempfile::NamedTempFile;
    use uuid::Uuid;

    fn setup_test_db() -> Connection {
        let temp_file = NamedTempFile::new().unwrap();
        let conn = Connection::open(temp_file.path()).unwrap();

        // Create analysis_requests table
        conn.execute(
            r#"
            CREATE TABLE analysis_requests (
                id TEXT PRIMARY KEY,
                session_id TEXT NOT NULL,
                prompt_template_id TEXT NOT NULL,
                template_variables TEXT NOT NULL,
                status TEXT NOT NULL,
                error_message TEXT,
                created_at TEXT NOT NULL,
                started_at TEXT,
                completed_at TEXT
            )
            "#,
            [],
        )
        .unwrap();

        conn
    }

    #[test]
    fn test_create_and_find_request() {
        let conn = setup_test_db();
        let repo = AnalysisRequestRepository::new(&conn);

        let session_id = Uuid::new_v4();
        let mut variables = HashMap::new();
        variables.insert("content".to_string(), "test content".to_string());

        let request = AnalysisRequest::new(session_id, "test_template".to_string(), variables);

        // Create request
        assert!(repo.create(&request).is_ok());

        // Find by ID
        let found = repo.find_by_id(&request.id).unwrap();
        assert!(found.is_some());
        let found_request = found.unwrap();
        assert_eq!(found_request.id, request.id);
        assert_eq!(found_request.status, RequestStatus::Queued);
    }

    #[test]
    fn test_queue_operations() {
        let conn = setup_test_db();
        let repo = AnalysisRequestRepository::new(&conn);

        let session_id = Uuid::new_v4();
        let request1 = AnalysisRequest::new(session_id, "template1".to_string(), HashMap::new());
        let request2 = AnalysisRequest::new(session_id, "template2".to_string(), HashMap::new());

        repo.create(&request1).unwrap();
        repo.create(&request2).unwrap();

        // Get queue
        let queue = repo.get_queue().unwrap();
        assert_eq!(queue.len(), 2);

        // Get next queued request
        let next = repo.get_next_queued_request().unwrap();
        assert!(next.is_some());

        // Mark as processing
        assert!(repo.mark_processing(&request1.id).unwrap());

        // Verify status change
        let updated = repo.find_by_id(&request1.id).unwrap().unwrap();
        assert_eq!(updated.status, RequestStatus::Processing);
        assert!(updated.started_at.is_some());

        // Queue should now have one less item
        let queue_after = repo.get_queue().unwrap();
        assert_eq!(queue_after.len(), 1);
    }

    #[test]
    fn test_status_transitions() {
        let conn = setup_test_db();
        let repo = AnalysisRequestRepository::new(&conn);

        let request =
            AnalysisRequest::new(Uuid::new_v4(), "test_template".to_string(), HashMap::new());

        repo.create(&request).unwrap();

        // Mark processing
        assert!(repo.mark_processing(&request.id).unwrap());
        let processing = repo.find_by_id(&request.id).unwrap().unwrap();
        assert_eq!(processing.status, RequestStatus::Processing);

        // Mark completed
        assert!(repo.mark_completed(&request.id).unwrap());
        let completed = repo.find_by_id(&request.id).unwrap().unwrap();
        assert_eq!(completed.status, RequestStatus::Completed);
        assert!(completed.completed_at.is_some());
    }

    #[test]
    fn test_mark_failed_and_retry() {
        let conn = setup_test_db();
        let repo = AnalysisRequestRepository::new(&conn);

        let request =
            AnalysisRequest::new(Uuid::new_v4(), "test_template".to_string(), HashMap::new());

        repo.create(&request).unwrap();
        repo.mark_processing(&request.id).unwrap();

        // Mark failed
        assert!(repo.mark_failed(&request.id, "API error").unwrap());
        let failed = repo.find_by_id(&request.id).unwrap().unwrap();
        assert_eq!(failed.status, RequestStatus::Failed);
        assert_eq!(failed.error_message, Some("API error".to_string()));

        // Retry failed request
        assert!(repo.retry_failed_request(&request.id).unwrap());
        let retried = repo.find_by_id(&request.id).unwrap().unwrap();
        assert_eq!(retried.status, RequestStatus::Queued);
        assert!(retried.error_message.is_none());
        assert!(retried.started_at.is_none());
        assert!(retried.completed_at.is_none());
    }

    #[test]
    fn test_find_by_session() {
        let conn = setup_test_db();
        let repo = AnalysisRequestRepository::new(&conn);

        let session_id = Uuid::new_v4();
        let other_session_id = Uuid::new_v4();

        let request1 = AnalysisRequest::new(session_id, "template1".to_string(), HashMap::new());
        let request2 = AnalysisRequest::new(session_id, "template2".to_string(), HashMap::new());
        let request3 =
            AnalysisRequest::new(other_session_id, "template3".to_string(), HashMap::new());

        repo.create(&request1).unwrap();
        repo.create(&request2).unwrap();
        repo.create(&request3).unwrap();

        let session_requests = repo.find_by_session_id(&session_id).unwrap();
        assert_eq!(session_requests.len(), 2);

        let other_session_requests = repo.find_by_session_id(&other_session_id).unwrap();
        assert_eq!(other_session_requests.len(), 1);
    }

    #[test]
    fn test_count_operations() {
        let conn = setup_test_db();
        let repo = AnalysisRequestRepository::new(&conn);

        let session_id = Uuid::new_v4();
        let request1 = AnalysisRequest::new(session_id, "template1".to_string(), HashMap::new());
        let request2 = AnalysisRequest::new(session_id, "template2".to_string(), HashMap::new());

        repo.create(&request1).unwrap();
        repo.create(&request2).unwrap();

        assert_eq!(repo.count_by_status(&RequestStatus::Queued).unwrap(), 2);
        assert_eq!(repo.count_by_status(&RequestStatus::Processing).unwrap(), 0);
        assert_eq!(repo.count_by_session(&session_id).unwrap(), 2);

        repo.mark_processing(&request1.id).unwrap();

        assert_eq!(repo.count_by_status(&RequestStatus::Queued).unwrap(), 1);
        assert_eq!(repo.count_by_status(&RequestStatus::Processing).unwrap(), 1);
    }

    #[test]
    fn test_queue_statistics() {
        let conn = setup_test_db();
        let repo = AnalysisRequestRepository::new(&conn);

        let request1 =
            AnalysisRequest::new(Uuid::new_v4(), "template1".to_string(), HashMap::new());
        let request2 =
            AnalysisRequest::new(Uuid::new_v4(), "template2".to_string(), HashMap::new());

        repo.create(&request1).unwrap();
        repo.create(&request2).unwrap();
        repo.mark_processing(&request1.id).unwrap();

        let stats = repo.get_queue_statistics().unwrap();
        assert_eq!(stats.queued_count, 1);
        assert_eq!(stats.processing_count, 1);
        assert_eq!(stats.total_pending(), 2);
    }

    #[test]
    fn test_find_duplicates() {
        let conn = setup_test_db();
        let repo = AnalysisRequestRepository::new(&conn);

        let session_id = Uuid::new_v4();
        let template_id = "duplicate_template";

        let request1 = AnalysisRequest::new(session_id, template_id.to_string(), HashMap::new());
        let request2 = AnalysisRequest::new(session_id, template_id.to_string(), HashMap::new());
        let request3 =
            AnalysisRequest::new(session_id, "other_template".to_string(), HashMap::new());

        repo.create(&request1).unwrap();
        repo.create(&request2).unwrap();
        repo.create(&request3).unwrap();

        let duplicates = repo
            .find_duplicate_requests(&session_id, template_id)
            .unwrap();
        assert_eq!(duplicates.len(), 2);
    }

    #[test]
    fn test_cleanup_completed() {
        let conn = setup_test_db();
        let repo = AnalysisRequestRepository::new(&conn);

        let request1 =
            AnalysisRequest::new(Uuid::new_v4(), "template1".to_string(), HashMap::new());
        let request2 =
            AnalysisRequest::new(Uuid::new_v4(), "template2".to_string(), HashMap::new());

        repo.create(&request1).unwrap();
        repo.create(&request2).unwrap();

        repo.mark_processing(&request1.id).unwrap();
        repo.mark_completed(&request1.id).unwrap();

        // Should clean up completed requests (using 0 days for immediate cleanup in test)
        let cleaned = repo.cleanup_completed_requests(0).unwrap();
        assert_eq!(cleaned, 1);

        // Verify request was deleted
        let found = repo.find_by_id(&request1.id).unwrap();
        assert!(found.is_none());

        // Queued request should still exist
        let queued_found = repo.find_by_id(&request2.id).unwrap();
        assert!(queued_found.is_some());
    }
}
