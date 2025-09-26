use crate::models::{AnalysisMetadata, AnalysisStatus, RetrospectionAnalysis};
use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, Row};
use serde::{Deserialize, Serialize};
use serde_json;
use uuid::Uuid;

pub struct RetrospectionAnalysisRepository<'a> {
    conn: &'a Connection,
}

impl<'a> RetrospectionAnalysisRepository<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    pub fn create(&self, analysis: &RetrospectionAnalysis) -> Result<()> {
        let metadata_json = serde_json::to_string(&analysis.metadata)
            .map_err(|e| anyhow!("Failed to serialize metadata: {}", e))?;

        self.conn.execute(
            r#"
            INSERT INTO retrospection_analyses (
                id, session_id, prompt_template_id, analysis_content,
                metadata, status, created_at, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
            "#,
            params![
                analysis.id.to_string(),
                analysis.session_id.to_string(),
                analysis.prompt_template_id,
                analysis.analysis_content,
                metadata_json,
                analysis.status.to_string(),
                analysis.created_at.to_rfc3339(),
                analysis.updated_at.to_rfc3339(),
            ],
        )?;

        Ok(())
    }

    pub fn find_by_id(&self, id: &Uuid) -> Result<Option<RetrospectionAnalysis>> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT id, session_id, prompt_template_id, analysis_content,
                   metadata, status, created_at, updated_at
            FROM retrospection_analyses
            WHERE id = ?1
            "#,
        )?;

        let mut rows = stmt.query_map(params![id.to_string()], |row| self.row_to_analysis(row))?;

        match rows.next() {
            Some(analysis) => Ok(Some(analysis?)),
            None => Ok(None),
        }
    }

    pub fn find_by_session_id(&self, session_id: &Uuid) -> Result<Vec<RetrospectionAnalysis>> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT id, session_id, prompt_template_id, analysis_content,
                   metadata, status, created_at, updated_at
            FROM retrospection_analyses
            WHERE session_id = ?1
            ORDER BY created_at DESC
            "#,
        )?;

        let analysis_iter = stmt.query_map(params![session_id.to_string()], |row| {
            self.row_to_analysis(row)
        })?;

        let mut analyses = Vec::new();
        for analysis in analysis_iter {
            analyses.push(analysis?);
        }

        Ok(analyses)
    }

    pub fn find_by_template_id(&self, template_id: &str) -> Result<Vec<RetrospectionAnalysis>> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT id, session_id, prompt_template_id, analysis_content,
                   metadata, status, created_at, updated_at
            FROM retrospection_analyses
            WHERE prompt_template_id = ?1
            ORDER BY created_at DESC
            "#,
        )?;

        let analysis_iter =
            stmt.query_map(params![template_id], |row| self.row_to_analysis(row))?;

        let mut analyses = Vec::new();
        for analysis in analysis_iter {
            analyses.push(analysis?);
        }

        Ok(analyses)
    }

    pub fn find_by_status(&self, status: &AnalysisStatus) -> Result<Vec<RetrospectionAnalysis>> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT id, session_id, prompt_template_id, analysis_content,
                   metadata, status, created_at, updated_at
            FROM retrospection_analyses
            WHERE status = ?1
            ORDER BY created_at DESC
            "#,
        )?;

        let analysis_iter =
            stmt.query_map(params![status.to_string()], |row| self.row_to_analysis(row))?;

        let mut analyses = Vec::new();
        for analysis in analysis_iter {
            analyses.push(analysis?);
        }

        Ok(analyses)
    }

    pub fn list_all(
        &self,
        limit: Option<u32>,
        offset: Option<u32>,
    ) -> Result<Vec<RetrospectionAnalysis>> {
        let limit_clause = limit.map(|l| format!("LIMIT {}", l)).unwrap_or_default();
        let offset_clause = offset.map(|o| format!("OFFSET {}", o)).unwrap_or_default();

        let query = format!(
            r#"
            SELECT id, session_id, prompt_template_id, analysis_content,
                   metadata, status, created_at, updated_at
            FROM retrospection_analyses
            ORDER BY created_at DESC
            {} {}
            "#,
            limit_clause, offset_clause
        );

        let mut stmt = self.conn.prepare(&query)?;
        let analysis_iter = stmt.query_map([], |row| self.row_to_analysis(row))?;

        let mut analyses = Vec::new();
        for analysis in analysis_iter {
            analyses.push(analysis?);
        }

        Ok(analyses)
    }

    pub fn update(&self, analysis: &RetrospectionAnalysis) -> Result<()> {
        let metadata_json = serde_json::to_string(&analysis.metadata)
            .map_err(|e| anyhow!("Failed to serialize metadata: {}", e))?;

        let rows_affected = self.conn.execute(
            r#"
            UPDATE retrospection_analyses
            SET session_id = ?2, prompt_template_id = ?3, analysis_content = ?4,
                metadata = ?5, status = ?6, updated_at = ?7
            WHERE id = ?1
            "#,
            params![
                analysis.id.to_string(),
                analysis.session_id.to_string(),
                analysis.prompt_template_id,
                analysis.analysis_content,
                metadata_json,
                analysis.status.to_string(),
                analysis.updated_at.to_rfc3339(),
            ],
        )?;

        if rows_affected == 0 {
            return Err(anyhow!("Analysis with id {} not found", analysis.id));
        }

        Ok(())
    }

    pub fn delete(&self, id: &Uuid) -> Result<bool> {
        let rows_affected = self.conn.execute(
            "DELETE FROM retrospection_analyses WHERE id = ?1",
            params![id.to_string()],
        )?;

        Ok(rows_affected > 0)
    }

    pub fn count_by_session(&self, session_id: &Uuid) -> Result<u32> {
        let mut stmt = self
            .conn
            .prepare("SELECT COUNT(*) FROM retrospection_analyses WHERE session_id = ?1")?;

        let count: i64 = stmt.query_row(params![session_id.to_string()], |row| row.get(0))?;

        Ok(count as u32)
    }

    pub fn count_by_status(&self, status: &AnalysisStatus) -> Result<u32> {
        let mut stmt = self
            .conn
            .prepare("SELECT COUNT(*) FROM retrospection_analyses WHERE status = ?1")?;

        let count: i64 = stmt.query_row(params![status.to_string()], |row| row.get(0))?;

        Ok(count as u32)
    }

    pub fn get_recent_analyses(&self, limit: u32) -> Result<Vec<RetrospectionAnalysis>> {
        self.list_all(Some(limit), None)
    }

    pub fn search_by_content(&self, search_term: &str) -> Result<Vec<RetrospectionAnalysis>> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT id, session_id, prompt_template_id, analysis_content,
                   metadata, status, created_at, updated_at
            FROM retrospection_analyses
            WHERE analysis_content LIKE ?1
            ORDER BY created_at DESC
            "#,
        )?;

        let search_pattern = format!("%{}%", search_term);
        let analysis_iter =
            stmt.query_map(params![search_pattern], |row| self.row_to_analysis(row))?;

        let mut analyses = Vec::new();
        for analysis in analysis_iter {
            analyses.push(analysis?);
        }

        Ok(analyses)
    }

    pub fn get_analysis_statistics(&self) -> Result<AnalysisStatistics> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT
                status,
                COUNT(*) as count,
                AVG(CAST(json_extract(metadata, '$.total_tokens') AS INTEGER)) as avg_tokens,
                AVG(CAST(json_extract(metadata, '$.estimated_cost') AS REAL)) as avg_cost,
                AVG(CAST(json_extract(metadata, '$.execution_time_ms') AS INTEGER)) as avg_execution_time
            FROM retrospection_analyses
            GROUP BY status
            "#,
        )?;

        let stat_iter = stmt.query_map([], |row| {
            Ok(StatusStatistics {
                status: row
                    .get::<_, String>(0)?
                    .parse()
                    .unwrap_or(AnalysisStatus::Draft),
                count: row.get::<_, i64>(1)? as u32,
                avg_tokens: row.get::<_, Option<f64>>(2).unwrap_or(Some(0.0)),
                avg_cost: row.get::<_, Option<f64>>(3).unwrap_or(Some(0.0)),
                avg_execution_time_ms: row.get::<_, Option<f64>>(4).unwrap_or(Some(0.0)),
            })
        })?;

        let mut status_stats = Vec::new();
        for stat in stat_iter {
            status_stats.push(stat?);
        }

        let total_count: u32 = status_stats.iter().map(|s| s.count).sum();
        let total_cost: f64 = status_stats
            .iter()
            .map(|s| s.avg_cost.unwrap_or(0.0) * s.count as f64)
            .sum();

        Ok(AnalysisStatistics {
            total_analyses: total_count,
            status_breakdown: status_stats,
            total_estimated_cost: total_cost,
        })
    }

    fn row_to_analysis(&self, row: &Row) -> rusqlite::Result<RetrospectionAnalysis> {
        let id_str: String = row.get(0)?;
        let session_id_str: String = row.get(1)?;
        let metadata_json: String = row.get(4)?;
        let status_str: String = row.get(5)?;
        let created_at_str: String = row.get(6)?;
        let updated_at_str: String = row.get(7)?;

        let id = Uuid::parse_str(&id_str).map_err(|_e| {
            rusqlite::Error::InvalidColumnType(0, "UUID".to_string(), rusqlite::types::Type::Text)
        })?;
        let session_id = Uuid::parse_str(&session_id_str).map_err(|_e| {
            rusqlite::Error::InvalidColumnType(1, "UUID".to_string(), rusqlite::types::Type::Text)
        })?;

        let metadata: AnalysisMetadata = serde_json::from_str(&metadata_json).map_err(|_e| {
            rusqlite::Error::InvalidColumnType(4, "JSON".to_string(), rusqlite::types::Type::Text)
        })?;

        let status: AnalysisStatus = status_str.parse().map_err(|_e| {
            rusqlite::Error::InvalidColumnType(
                5,
                "AnalysisStatus".to_string(),
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

        let updated_at = DateTime::parse_from_rfc3339(&updated_at_str)
            .map_err(|_e| {
                rusqlite::Error::InvalidColumnType(
                    7,
                    "DateTime".to_string(),
                    rusqlite::types::Type::Text,
                )
            })?
            .with_timezone(&Utc);

        Ok(RetrospectionAnalysis {
            id,
            session_id,
            prompt_template_id: row.get(2)?,
            analysis_content: row.get(3)?,
            metadata,
            status,
            created_at,
            updated_at,
        })
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AnalysisStatistics {
    pub total_analyses: u32,
    pub status_breakdown: Vec<StatusStatistics>,
    pub total_estimated_cost: f64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct StatusStatistics {
    pub status: AnalysisStatus,
    pub count: u32,
    pub avg_tokens: Option<f64>,
    pub avg_cost: Option<f64>,
    pub avg_execution_time_ms: Option<f64>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::RetrospectionAnalysis;
    use rusqlite::Connection;
    use tempfile::NamedTempFile;

    fn setup_test_db() -> Connection {
        let temp_file = NamedTempFile::new().unwrap();
        let conn = Connection::open(temp_file.path()).unwrap();

        // Create retrospection_analyses table
        conn.execute(
            r#"
            CREATE TABLE retrospection_analyses (
                id TEXT PRIMARY KEY,
                session_id TEXT NOT NULL,
                prompt_template_id TEXT NOT NULL,
                analysis_content TEXT NOT NULL,
                metadata TEXT NOT NULL,
                status TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            )
            "#,
            [],
        )
        .unwrap();

        conn
    }

    #[test]
    fn test_create_and_find_analysis() {
        let conn = setup_test_db();
        let repo = RetrospectionAnalysisRepository::new(&conn);

        let mut analysis = RetrospectionAnalysis::new(Uuid::new_v4(), "test_template".to_string());
        analysis.complete(
            "Test analysis content".to_string(),
            AnalysisMetadata::default(),
        );

        // Create analysis
        assert!(repo.create(&analysis).is_ok());

        // Find by ID
        let found = repo.find_by_id(&analysis.id).unwrap();
        assert!(found.is_some());
        let found_analysis = found.unwrap();
        assert_eq!(found_analysis.id, analysis.id);
        assert_eq!(found_analysis.analysis_content, analysis.analysis_content);
    }

    #[test]
    fn test_find_by_session_id() {
        let conn = setup_test_db();
        let repo = RetrospectionAnalysisRepository::new(&conn);

        let session_id = Uuid::new_v4();
        let mut analysis1 = RetrospectionAnalysis::new(session_id, "template1".to_string());
        analysis1.complete("Analysis 1".to_string(), AnalysisMetadata::default());

        let mut analysis2 = RetrospectionAnalysis::new(session_id, "template2".to_string());
        analysis2.complete("Analysis 2".to_string(), AnalysisMetadata::default());

        repo.create(&analysis1).unwrap();
        repo.create(&analysis2).unwrap();

        let analyses = repo.find_by_session_id(&session_id).unwrap();
        assert_eq!(analyses.len(), 2);
    }

    #[test]
    fn test_update_analysis() {
        let conn = setup_test_db();
        let repo = RetrospectionAnalysisRepository::new(&conn);

        let mut analysis = RetrospectionAnalysis::new(Uuid::new_v4(), "test_template".to_string());
        analysis.complete("Original content".to_string(), AnalysisMetadata::default());

        repo.create(&analysis).unwrap();

        // Update analysis
        analysis.analysis_content = "Updated content".to_string();
        analysis.updated_at = chrono::Utc::now();

        assert!(repo.update(&analysis).is_ok());

        // Verify update
        let updated = repo.find_by_id(&analysis.id).unwrap().unwrap();
        assert_eq!(updated.analysis_content, "Updated content");
    }

    #[test]
    fn test_delete_analysis() {
        let conn = setup_test_db();
        let repo = RetrospectionAnalysisRepository::new(&conn);

        let mut analysis = RetrospectionAnalysis::new(Uuid::new_v4(), "test_template".to_string());
        analysis.complete("Test content".to_string(), AnalysisMetadata::default());

        repo.create(&analysis).unwrap();
        assert!(repo.delete(&analysis.id).unwrap());

        let found = repo.find_by_id(&analysis.id).unwrap();
        assert!(found.is_none());
    }

    #[test]
    fn test_count_operations() {
        let conn = setup_test_db();
        let repo = RetrospectionAnalysisRepository::new(&conn);

        let session_id = Uuid::new_v4();
        let mut analysis1 = RetrospectionAnalysis::new(session_id, "template1".to_string());
        // Keep analysis1 in draft state

        let mut analysis2 = RetrospectionAnalysis::new(session_id, "template2".to_string());
        analysis2.complete(
            "Completed analysis".to_string(),
            AnalysisMetadata::default(),
        );

        repo.create(&analysis1).unwrap();
        repo.create(&analysis2).unwrap();

        assert_eq!(repo.count_by_session(&session_id).unwrap(), 2);
        assert_eq!(repo.count_by_status(&AnalysisStatus::Draft).unwrap(), 1);
        assert_eq!(repo.count_by_status(&AnalysisStatus::Complete).unwrap(), 1);
    }

    #[test]
    fn test_search_by_content() {
        let conn = setup_test_db();
        let repo = RetrospectionAnalysisRepository::new(&conn);

        let mut analysis1 = RetrospectionAnalysis::new(Uuid::new_v4(), "template1".to_string());
        analysis1.complete(
            "This analysis contains important insights".to_string(),
            AnalysisMetadata::default(),
        );

        let mut analysis2 = RetrospectionAnalysis::new(Uuid::new_v4(), "template2".to_string());
        analysis2.complete(
            "This is a different type of analysis".to_string(),
            AnalysisMetadata::default(),
        );

        repo.create(&analysis1).unwrap();
        repo.create(&analysis2).unwrap();

        let results = repo.search_by_content("important").unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, analysis1.id);

        let all_results = repo.search_by_content("analysis").unwrap();
        assert_eq!(all_results.len(), 2);
    }
}
