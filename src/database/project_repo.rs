use anyhow::{Context, Result};
use rusqlite::{params, Row};
use uuid::Uuid;

use crate::database::connection::DatabaseManager;
use crate::models::Project;

pub struct ProjectRepository {
    db_manager: DatabaseManager,
}

impl ProjectRepository {
    pub fn new(db_manager: DatabaseManager) -> Self {
        Self { db_manager }
    }

    pub fn create(&self, project: &Project) -> Result<()> {
        self.db_manager.with_transaction(|conn| {
            conn.execute(
                "INSERT INTO projects (id, name, description, working_directory, created_at, updated_at, session_count, total_tokens)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    project.id.to_string(),
                    project.name,
                    project.description,
                    project.working_directory.as_ref().map(|p| p.to_string_lossy().to_string()),
                    project.created_at.to_rfc3339(),
                    project.updated_at.to_rfc3339(),
                    project.session_count,
                    project.total_tokens,
                ],
            )?;
            Ok(())
        })
        .with_context(|| format!("Failed to insert project: {}", project.name))
    }

    pub fn get_by_name(&self, name: &str) -> Result<Option<Project>> {
        self.db_manager.with_connection(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, name, description, working_directory, created_at, updated_at, session_count, total_tokens
                 FROM projects WHERE name = ?1"
            )?;

            let mut rows = stmt.query_map(params![name], |row| {
                self.map_row_to_project(row)
            })?;

            if let Some(result) = rows.next() {
                Ok(Some(result?))
            } else {
                Ok(None)
            }
        })
    }

    pub fn get_by_id(&self, id: &Uuid) -> Result<Option<Project>> {
        self.db_manager.with_connection(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, name, description, working_directory, created_at, updated_at, session_count, total_tokens
                 FROM projects WHERE id = ?1"
            )?;

            let mut rows = stmt.query_map(params![id.to_string()], |row| {
                self.map_row_to_project(row)
            })?;

            if let Some(result) = rows.next() {
                Ok(Some(result?))
            } else {
                Ok(None)
            }
        })
    }

    pub fn get_all(&self) -> Result<Vec<Project>> {
        self.db_manager.with_connection(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, name, description, working_directory, created_at, updated_at, session_count, total_tokens
                 FROM projects ORDER BY updated_at DESC"
            )?;

            let rows = stmt.query_map([], |row| {
                self.map_row_to_project(row)
            })?;

            let mut projects = Vec::new();
            for row in rows {
                projects.push(row?);
            }

            Ok(projects)
        })
    }

    pub fn update(&self, project: &Project) -> Result<()> {
        self.db_manager.with_transaction(|conn| {
            conn.execute(
                "UPDATE projects
                 SET description = ?1, working_directory = ?2, updated_at = ?3, session_count = ?4, total_tokens = ?5
                 WHERE id = ?6",
                params![
                    project.description,
                    project.working_directory.as_ref().map(|p| p.to_string_lossy().to_string()),
                    project.updated_at.to_rfc3339(),
                    project.session_count,
                    project.total_tokens,
                    project.id.to_string(),
                ],
            )?;
            Ok(())
        })
        .with_context(|| format!("Failed to update project: {}", project.name))
    }

    pub fn delete(&self, id: &Uuid) -> Result<()> {
        self.db_manager
            .with_transaction(|conn| {
                conn.execute(
                    "DELETE FROM projects WHERE id = ?1",
                    params![id.to_string()],
                )?;
                Ok(())
            })
            .with_context(|| format!("Failed to delete project: {id}"))
    }

    pub fn create_if_not_exists(&self, name: &str, description: Option<String>) -> Result<Project> {
        if let Some(existing) = self.get_by_name(name)? {
            return Ok(existing);
        }

        let mut project = Project::new(name.to_string());
        if let Some(desc) = description {
            project = project.with_description(desc);
        }
        self.create(&project)?;
        Ok(project)
    }

    fn parse_timestamp(&self, timestamp_str: &str) -> Result<chrono::DateTime<chrono::Utc>> {
        // Try different timestamp formats
        let formats = [
            "%Y-%m-%dT%H:%M:%S%.fZ",  // RFC3339 with microseconds
            "%Y-%m-%dT%H:%M:%SZ",     // RFC3339 without microseconds
            "%Y-%m-%dT%H:%M:%S%.f%z", // ISO 8601 with timezone
            "%Y-%m-%dT%H:%M:%S%z",    // ISO 8601 without microseconds
            "%Y-%m-%d %H:%M:%S",      // SQLite CURRENT_TIMESTAMP format
        ];

        // First try RFC3339
        if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(timestamp_str) {
            return Ok(dt.with_timezone(&chrono::Utc));
        }

        // Try different formats
        for format in &formats {
            if let Ok(dt) = chrono::DateTime::parse_from_str(timestamp_str, format) {
                return Ok(dt.with_timezone(&chrono::Utc));
            }
        }

        // Try parsing SQLite CURRENT_TIMESTAMP format as UTC directly
        if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(timestamp_str, "%Y-%m-%d %H:%M:%S") {
            return Ok(chrono::DateTime::from_naive_utc_and_offset(dt, chrono::Utc));
        }

        // Fallback: try to parse as Utc directly
        if let Ok(dt) = timestamp_str.parse::<chrono::DateTime<chrono::Utc>>() {
            return Ok(dt);
        }

        Err(anyhow::anyhow!(
            "Unable to parse timestamp: {timestamp_str}"
        ))
    }

    fn map_row_to_project(&self, row: &Row) -> rusqlite::Result<Project> {
        let id_str: String = row.get(0)?;
        let id = Uuid::parse_str(&id_str).map_err(|_| {
            rusqlite::Error::InvalidColumnType(0, "id".to_string(), rusqlite::types::Type::Text)
        })?;

        let created_at_str: String = row.get(4)?;
        let updated_at_str: String = row.get(5)?;

        let created_at = self.parse_timestamp(&created_at_str).map_err(|_| {
            rusqlite::Error::InvalidColumnType(
                4,
                "created_at".to_string(),
                rusqlite::types::Type::Text,
            )
        })?;

        let updated_at = self.parse_timestamp(&updated_at_str).map_err(|_| {
            rusqlite::Error::InvalidColumnType(
                5,
                "updated_at".to_string(),
                rusqlite::types::Type::Text,
            )
        })?;

        let working_directory: Option<String> = row.get(3)?;
        let working_directory = working_directory.map(std::path::PathBuf::from);

        Ok(Project {
            id,
            name: row.get(1)?,
            description: row.get(2)?,
            working_directory,
            created_at,
            updated_at,
            session_count: row.get(6)?,
            total_tokens: row.get(7)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::schema;

    #[test]
    fn test_create_and_get_project() {
        let db_manager = DatabaseManager::new(":memory:").unwrap();
        db_manager.with_connection(schema::create_schema).unwrap();

        let repo = ProjectRepository::new(db_manager);

        let project =
            Project::new("Test Project".to_string()).with_description("A test project".to_string());

        repo.create(&project).unwrap();

        let retrieved = repo.get_by_name("Test Project").unwrap().unwrap();
        assert_eq!(retrieved.name, "Test Project");
        assert_eq!(retrieved.description, Some("A test project".to_string()));
    }

    #[test]
    fn test_create_if_not_exists() {
        let db_manager = DatabaseManager::new(":memory:").unwrap();
        db_manager.with_connection(schema::create_schema).unwrap();

        let repo = ProjectRepository::new(db_manager);

        // First call should create
        let project1 = repo
            .create_if_not_exists("Test Project", Some("Description".to_string()))
            .unwrap();

        // Second call should return existing
        let project2 = repo
            .create_if_not_exists("Test Project", Some("Different Description".to_string()))
            .unwrap();

        assert_eq!(project1.id, project2.id);
        assert_eq!(project1.description, project2.description); // Should keep original description
    }
}
