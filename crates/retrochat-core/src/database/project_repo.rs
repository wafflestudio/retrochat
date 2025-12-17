use anyhow::{Context, Result as AnyhowResult};
use chrono::{DateTime, Utc};
use sqlx::{sqlite::SqliteRow, Pool, Row, Sqlite};
use std::path::PathBuf;
use uuid::Uuid;

use super::connection::DatabaseManager;
use crate::models::Project;

pub struct ProjectRepository {
    pool: Pool<Sqlite>,
}

impl ProjectRepository {
    pub fn new(db: &DatabaseManager) -> Self {
        Self {
            pool: db.pool().clone(),
        }
    }

    pub async fn create(&self, project: &Project) -> AnyhowResult<()> {
        sqlx::query(
            r#"
            INSERT INTO projects (id, name, description, working_directory, created_at, updated_at, session_count, total_tokens)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            "#
        )
        .bind(project.id.to_string())
        .bind(&project.name)
        .bind(project.description.as_ref())
        .bind(project.working_directory.as_ref().map(|p| p.to_string_lossy().to_string()))
        .bind(project.created_at.to_rfc3339())
        .bind(project.updated_at.to_rfc3339())
        .bind(project.session_count)
        .bind(project.total_tokens as i64)
        .execute(&self.pool)
        .await
        .context("Failed to create project")?;

        Ok(())
    }

    pub async fn get_by_name(&self, name: &str) -> AnyhowResult<Option<Project>> {
        let row = sqlx::query(
            r#"
            SELECT id, name, description, working_directory, created_at, updated_at, session_count, total_tokens
            FROM projects WHERE name = ?
            "#
        )
        .bind(name)
        .fetch_optional(&self.pool)
        .await
        .context("Failed to fetch project by name")?;

        match row {
            Some(row) => {
                let project = self.row_to_project(&row)?;
                Ok(Some(project))
            }
            None => Ok(None),
        }
    }

    pub async fn get_by_id(&self, id: &Uuid) -> AnyhowResult<Option<Project>> {
        let row = sqlx::query(
            r#"
            SELECT id, name, description, working_directory, created_at, updated_at, session_count, total_tokens
            FROM projects WHERE id = ?
            "#
        )
        .bind(id.to_string())
        .fetch_optional(&self.pool)
        .await
        .context("Failed to fetch project by ID")?;

        match row {
            Some(row) => {
                let project = self.row_to_project(&row)?;
                Ok(Some(project))
            }
            None => Ok(None),
        }
    }

    pub async fn get_all(&self) -> AnyhowResult<Vec<Project>> {
        let rows = sqlx::query(
            r#"
            SELECT id, name, description, working_directory, created_at, updated_at, session_count, total_tokens
            FROM projects ORDER BY updated_at DESC
            "#
        )
        .fetch_all(&self.pool)
        .await
        .context("Failed to fetch all projects")?;

        let mut projects = Vec::new();
        for row in rows {
            let project = self.row_to_project(&row)?;
            projects.push(project);
        }

        Ok(projects)
    }

    pub async fn update(&self, project: &Project) -> AnyhowResult<()> {
        sqlx::query(
            r#"
            UPDATE projects 
            SET name = ?, description = ?, working_directory = ?, updated_at = ?, 
                session_count = ?, total_tokens = ?
            WHERE id = ?
            "#,
        )
        .bind(&project.name)
        .bind(project.description.as_ref())
        .bind(
            project
                .working_directory
                .as_ref()
                .map(|p| p.to_string_lossy().to_string()),
        )
        .bind(project.updated_at.to_rfc3339())
        .bind(project.session_count)
        .bind(project.total_tokens as i64)
        .bind(project.id.to_string())
        .execute(&self.pool)
        .await
        .context("Failed to update project")?;

        Ok(())
    }

    pub async fn delete(&self, id: &Uuid) -> AnyhowResult<()> {
        sqlx::query("DELETE FROM projects WHERE id = ?")
            .bind(id.to_string())
            .execute(&self.pool)
            .await
            .context("Failed to delete project")?;

        Ok(())
    }

    pub async fn count(&self) -> AnyhowResult<i64> {
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM projects")
            .fetch_one(&self.pool)
            .await
            .context("Failed to count projects")?;

        Ok(count)
    }

    pub async fn exists_by_name(&self, name: &str) -> AnyhowResult<bool> {
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM projects WHERE name = ?")
            .bind(name)
            .fetch_one(&self.pool)
            .await
            .context("Failed to check project existence")?;

        Ok(count > 0)
    }

    pub async fn create_if_not_exists(
        &self,
        name: &str,
        description: Option<&str>,
    ) -> AnyhowResult<()> {
        if !self.exists_by_name(name).await? {
            let project = Project::new(name.to_string())
                .with_description(description.unwrap_or("").to_string());
            self.create(&project).await?;
        }
        Ok(())
    }

    pub async fn get_by_working_directory(
        &self,
        working_directory: &std::path::Path,
    ) -> AnyhowResult<Vec<Project>> {
        let dir_str = working_directory.to_string_lossy().to_string();
        let rows = sqlx::query(
            r#"
            SELECT id, name, description, working_directory, created_at, updated_at, session_count, total_tokens
            FROM projects WHERE working_directory = ?
            ORDER BY updated_at DESC
            "#
        )
        .bind(dir_str)
        .fetch_all(&self.pool)
        .await
        .context("Failed to fetch projects by working directory")?;

        let mut projects = Vec::new();
        for row in rows {
            let project = self.row_to_project(&row)?;
            projects.push(project);
        }

        Ok(projects)
    }

    fn row_to_project(&self, row: &SqliteRow) -> AnyhowResult<Project> {
        let id_str: String = row.try_get("id")?;
        let name: String = row.try_get("name")?;
        let description: Option<String> = row.try_get("description")?;
        let working_directory_str: Option<String> = row.try_get("working_directory")?;
        let created_at_str: String = row.try_get("created_at")?;
        let updated_at_str: String = row.try_get("updated_at")?;
        let session_count: i64 = row.try_get("session_count")?;
        let total_tokens: i64 = row.try_get("total_tokens")?;

        let id = Uuid::parse_str(&id_str).context("Invalid project ID format")?;

        let working_directory = working_directory_str.map(PathBuf::from);

        let created_at = DateTime::parse_from_rfc3339(&created_at_str)
            .context("Invalid created_at timestamp format")?
            .with_timezone(&Utc);

        let updated_at = DateTime::parse_from_rfc3339(&updated_at_str)
            .context("Invalid updated_at timestamp format")?
            .with_timezone(&Utc);

        Ok(Project {
            id,
            name,
            description,
            working_directory,
            created_at,
            updated_at,
            session_count: session_count as u32,
            total_tokens: total_tokens as u64,
        })
    }
}
