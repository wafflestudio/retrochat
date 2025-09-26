use crate::models::{PromptTemplate, PromptVariable};
use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, Row};
use uuid::Uuid;

pub struct PromptTemplateRepository<'a> {
    conn: &'a Connection,
}

impl<'a> PromptTemplateRepository<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    pub fn create(&self, template: &PromptTemplate) -> Result<()> {
        let tx = self.conn.unchecked_transaction()?;

        // Insert the template
        tx.execute(
            r#"
            INSERT INTO prompt_templates (
                id, name, description, template, category,
                is_default, created_at, modified_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
            "#,
            params![
                template.id,
                template.name,
                template.description,
                template.template,
                template.category,
                template.is_default,
                template.created_at.to_rfc3339(),
                template.modified_at.to_rfc3339(),
            ],
        )?;

        // Insert variables
        for variable in &template.variables {
            tx.execute(
                r#"
                INSERT INTO prompt_variables (
                    id, template_id, name, description, required, default_value
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                "#,
                params![
                    Uuid::new_v4().to_string(),
                    template.id,
                    variable.name,
                    variable.description,
                    variable.required,
                    variable.default_value,
                ],
            )?;
        }

        tx.commit()?;
        Ok(())
    }

    pub fn find_by_id(&self, id: &str) -> Result<Option<PromptTemplate>> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT id, name, description, template, category,
                   is_default, created_at, modified_at
            FROM prompt_templates
            WHERE id = ?1
            "#,
        )?;

        let mut rows = stmt.query_map(params![id], |row| self.row_to_template_basic(row))?;

        match rows.next() {
            Some(template_result) => {
                let mut template = template_result?;
                template.variables = self.load_variables_for_template(&template.id)?;
                Ok(Some(template))
            }
            None => Ok(None),
        }
    }

    pub fn find_by_category(&self, category: &str) -> Result<Vec<PromptTemplate>> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT id, name, description, template, category,
                   is_default, created_at, modified_at
            FROM prompt_templates
            WHERE category = ?1
            ORDER BY is_default DESC, name ASC
            "#,
        )?;

        let template_iter =
            stmt.query_map(params![category], |row| self.row_to_template_basic(row))?;

        let mut templates = Vec::new();
        for template_result in template_iter {
            let mut template = template_result?;
            template.variables = self.load_variables_for_template(&template.id)?;
            templates.push(template);
        }

        Ok(templates)
    }

    pub fn list_all(&self, include_default: bool) -> Result<Vec<PromptTemplate>> {
        let query = if include_default {
            r#"
            SELECT id, name, description, template, category,
                   is_default, created_at, modified_at
            FROM prompt_templates
            ORDER BY is_default DESC, category ASC, name ASC
            "#
        } else {
            r#"
            SELECT id, name, description, template, category,
                   is_default, created_at, modified_at
            FROM prompt_templates
            WHERE is_default = false
            ORDER BY category ASC, name ASC
            "#
        };

        let mut stmt = self.conn.prepare(query)?;
        let template_iter = stmt.query_map([], |row| self.row_to_template_basic(row))?;

        let mut templates = Vec::new();
        for template_result in template_iter {
            let mut template = template_result?;
            template.variables = self.load_variables_for_template(&template.id)?;
            templates.push(template);
        }

        Ok(templates)
    }

    pub fn list_default_templates(&self) -> Result<Vec<PromptTemplate>> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT id, name, description, template, category,
                   is_default, created_at, modified_at
            FROM prompt_templates
            WHERE is_default = true
            ORDER BY category ASC, name ASC
            "#,
        )?;

        let template_iter = stmt.query_map([], |row| self.row_to_template_basic(row))?;

        let mut templates = Vec::new();
        for template_result in template_iter {
            let mut template = template_result?;
            template.variables = self.load_variables_for_template(&template.id)?;
            templates.push(template);
        }

        Ok(templates)
    }

    pub fn update(&self, template: &PromptTemplate) -> Result<()> {
        if template.is_default {
            return Err(anyhow!("Cannot update default template"));
        }

        let tx = self.conn.unchecked_transaction()?;

        // Update the template
        let rows_affected = tx.execute(
            r#"
            UPDATE prompt_templates
            SET name = ?2, description = ?3, template = ?4, category = ?5, modified_at = ?6
            WHERE id = ?1 AND is_default = false
            "#,
            params![
                template.id,
                template.name,
                template.description,
                template.template,
                template.category,
                template.modified_at.to_rfc3339(),
            ],
        )?;

        if rows_affected == 0 {
            tx.rollback()?;
            return Err(anyhow!("Template not found or is a default template"));
        }

        // Remove existing variables
        tx.execute(
            "DELETE FROM prompt_variables WHERE template_id = ?1",
            params![template.id],
        )?;

        // Insert updated variables
        for variable in &template.variables {
            tx.execute(
                r#"
                INSERT INTO prompt_variables (
                    id, template_id, name, description, required, default_value
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                "#,
                params![
                    Uuid::new_v4().to_string(),
                    template.id,
                    variable.name,
                    variable.description,
                    variable.required,
                    variable.default_value,
                ],
            )?;
        }

        tx.commit()?;
        Ok(())
    }

    pub fn delete(&self, id: &str) -> Result<bool> {
        // Check if it's a default template
        let is_default: bool = self
            .conn
            .query_row(
                "SELECT is_default FROM prompt_templates WHERE id = ?1",
                params![id],
                |row| row.get(0),
            )
            .unwrap_or(false);

        if is_default {
            return Err(anyhow!("Cannot delete default template"));
        }

        let tx = self.conn.unchecked_transaction()?;

        // Delete variables first (foreign key constraint)
        tx.execute(
            "DELETE FROM prompt_variables WHERE template_id = ?1",
            params![id],
        )?;

        // Delete template
        let rows_affected = tx.execute(
            "DELETE FROM prompt_templates WHERE id = ?1 AND is_default = false",
            params![id],
        )?;

        tx.commit()?;
        Ok(rows_affected > 0)
    }

    pub fn exists(&self, id: &str) -> Result<bool> {
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM prompt_templates WHERE id = ?1",
            params![id],
            |row| row.get(0),
        )?;

        Ok(count > 0)
    }

    pub fn count_by_category(&self, category: &str) -> Result<u32> {
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM prompt_templates WHERE category = ?1",
            params![category],
            |row| row.get(0),
        )?;

        Ok(count as u32)
    }

    pub fn get_categories(&self) -> Result<Vec<String>> {
        let mut stmt = self
            .conn
            .prepare("SELECT DISTINCT category FROM prompt_templates ORDER BY category")?;

        let category_iter = stmt.query_map([], |row| Ok(row.get::<_, String>(0)?))?;

        let mut categories = Vec::new();
        for category in category_iter {
            categories.push(category?);
        }

        Ok(categories)
    }

    pub fn search_templates(&self, search_term: &str) -> Result<Vec<PromptTemplate>> {
        let search_pattern = format!("%{}%", search_term);
        let mut stmt = self.conn.prepare(
            r#"
            SELECT id, name, description, template, category,
                   is_default, created_at, modified_at
            FROM prompt_templates
            WHERE name LIKE ?1 OR description LIKE ?1 OR template LIKE ?1
            ORDER BY is_default DESC, name ASC
            "#,
        )?;

        let template_iter = stmt.query_map(params![search_pattern], |row| {
            self.row_to_template_basic(row)
        })?;

        let mut templates = Vec::new();
        for template_result in template_iter {
            let mut template = template_result?;
            template.variables = self.load_variables_for_template(&template.id)?;
            templates.push(template);
        }

        Ok(templates)
    }

    pub fn validate_template_usage(&self, template_id: &str) -> Result<bool> {
        // Check if template is used in any analyses or analysis requests
        let analysis_count: i64 = self
            .conn
            .query_row(
                "SELECT COUNT(*) FROM retrospection_analyses WHERE prompt_template_id = ?1",
                params![template_id],
                |row| row.get(0),
            )
            .unwrap_or(0);

        let request_count: i64 = self
            .conn
            .query_row(
                "SELECT COUNT(*) FROM analysis_requests WHERE prompt_template_id = ?1",
                params![template_id],
                |row| row.get(0),
            )
            .unwrap_or(0);

        Ok(analysis_count == 0 && request_count == 0)
    }

    pub fn clone_template(&self, source_id: &str, new_id: &str, new_name: &str) -> Result<()> {
        let source_template = self
            .find_by_id(source_id)?
            .ok_or_else(|| anyhow!("Source template not found"))?;

        if self.exists(new_id)? {
            return Err(anyhow!("Template with ID '{}' already exists", new_id));
        }

        let cloned_template = PromptTemplate::new(
            new_id,
            new_name,
            &format!("Copy of {}", source_template.description),
            &source_template.template,
            source_template.variables.clone(),
            &source_template.category,
        );

        self.create(&cloned_template)?;
        Ok(())
    }

    fn load_variables_for_template(&self, template_id: &str) -> Result<Vec<PromptVariable>> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT name, description, required, default_value
            FROM prompt_variables
            WHERE template_id = ?1
            ORDER BY name
            "#,
        )?;

        let variable_iter = stmt.query_map(params![template_id], |row| {
            Ok(PromptVariable {
                name: row.get(0)?,
                description: row.get(1)?,
                required: row.get(2)?,
                default_value: row.get(3)?,
            })
        })?;

        let mut variables = Vec::new();
        for variable in variable_iter {
            variables.push(variable?);
        }

        Ok(variables)
    }

    fn row_to_template_basic(&self, row: &Row) -> rusqlite::Result<PromptTemplate> {
        let created_at_str: String = row.get(6)?;
        let modified_at_str: String = row.get(7)?;

        let created_at = DateTime::parse_from_rfc3339(&created_at_str)
            .map_err(|_e| {
                rusqlite::Error::InvalidColumnType(
                    6,
                    "DateTime".to_string(),
                    rusqlite::types::Type::Text,
                )
            })?
            .with_timezone(&Utc);

        let modified_at = DateTime::parse_from_rfc3339(&modified_at_str)
            .map_err(|_e| {
                rusqlite::Error::InvalidColumnType(
                    7,
                    "DateTime".to_string(),
                    rusqlite::types::Type::Text,
                )
            })?
            .with_timezone(&Utc);

        Ok(PromptTemplate {
            id: row.get(0)?,
            name: row.get(1)?,
            description: row.get(2)?,
            template: row.get(3)?,
            category: row.get(4)?,
            is_default: row.get(5)?,
            created_at,
            modified_at,
            variables: Vec::new(), // Will be loaded separately
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{PromptTemplate, PromptVariable};
    use rusqlite::Connection;
    use tempfile::NamedTempFile;

    fn setup_test_db() -> Connection {
        let temp_file = NamedTempFile::new().unwrap();
        let conn = Connection::open(temp_file.path()).unwrap();

        // Create prompt_templates table
        conn.execute(
            r#"
            CREATE TABLE prompt_templates (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                description TEXT NOT NULL,
                template TEXT NOT NULL,
                category TEXT NOT NULL,
                is_default BOOLEAN NOT NULL DEFAULT false,
                created_at TEXT NOT NULL,
                modified_at TEXT NOT NULL
            )
            "#,
            [],
        )
        .unwrap();

        // Create prompt_variables table
        conn.execute(
            r#"
            CREATE TABLE prompt_variables (
                id TEXT PRIMARY KEY,
                template_id TEXT NOT NULL,
                name TEXT NOT NULL,
                description TEXT NOT NULL,
                required BOOLEAN NOT NULL DEFAULT false,
                default_value TEXT,
                FOREIGN KEY (template_id) REFERENCES prompt_templates (id)
            )
            "#,
            [],
        )
        .unwrap();

        // Create tables for usage validation tests
        conn.execute(
            r#"
            CREATE TABLE retrospection_analyses (
                id TEXT PRIMARY KEY,
                prompt_template_id TEXT NOT NULL
            )
            "#,
            [],
        )
        .unwrap();

        conn.execute(
            r#"
            CREATE TABLE analysis_requests (
                id TEXT PRIMARY KEY,
                prompt_template_id TEXT NOT NULL
            )
            "#,
            [],
        )
        .unwrap();

        conn
    }

    #[test]
    fn test_create_and_find_template() {
        let conn = setup_test_db();
        let repo = PromptTemplateRepository::new(&conn);

        let variables = vec![
            PromptVariable::required("chat_content", "Chat content to analyze"),
            PromptVariable::optional("focus", "Analysis focus", "general"),
        ];

        let template = PromptTemplate::new(
            "test_template",
            "Test Template",
            "Template for testing",
            "Analyze: {chat_content} with focus on {focus}",
            variables,
            "test",
        );

        // Create template
        assert!(repo.create(&template).is_ok());

        // Find by ID
        let found = repo.find_by_id("test_template").unwrap();
        assert!(found.is_some());
        let found_template = found.unwrap();
        assert_eq!(found_template.id, "test_template");
        assert_eq!(found_template.variables.len(), 2);
        assert!(!found_template.is_default);
    }

    #[test]
    fn test_list_templates() {
        let conn = setup_test_db();
        let repo = PromptTemplateRepository::new(&conn);

        let template1 = PromptTemplate::new(
            "template1",
            "Template 1",
            "First template",
            "Content: {content}",
            vec![PromptVariable::required("content", "Content")],
            "analysis",
        );

        let template2 = PromptTemplate::default_template(
            "template2",
            "Template 2",
            "Second template",
            "Content: {content}",
            vec![PromptVariable::required("content", "Content")],
            "analysis",
        );

        repo.create(&template1).unwrap();
        repo.create(&template2).unwrap();

        // List all templates
        let all_templates = repo.list_all(true).unwrap();
        assert_eq!(all_templates.len(), 2);

        // List only custom templates
        let custom_templates = repo.list_all(false).unwrap();
        assert_eq!(custom_templates.len(), 1);
        assert_eq!(custom_templates[0].id, "template1");

        // List only default templates
        let default_templates = repo.list_default_templates().unwrap();
        assert_eq!(default_templates.len(), 1);
        assert_eq!(default_templates[0].id, "template2");
    }

    #[test]
    fn test_update_template() {
        let conn = setup_test_db();
        let repo = PromptTemplateRepository::new(&conn);

        let mut template = PromptTemplate::new(
            "updatable_template",
            "Original Name",
            "Original description",
            "Original: {content}",
            vec![PromptVariable::required("content", "Content")],
            "test",
        );

        repo.create(&template).unwrap();

        // Update template
        let new_variables = vec![
            PromptVariable::required("content", "Updated content"),
            PromptVariable::optional("style", "Style", "formal"),
        ];

        template
            .update(
                Some("Updated Name".to_string()),
                Some("Updated description".to_string()),
                Some("Updated: {content} in {style} style".to_string()),
                Some(new_variables),
                Some("updated".to_string()),
            )
            .unwrap();

        assert!(repo.update(&template).is_ok());

        // Verify update
        let updated = repo.find_by_id("updatable_template").unwrap().unwrap();
        assert_eq!(updated.name, "Updated Name");
        assert_eq!(updated.variables.len(), 2);
    }

    #[test]
    fn test_cannot_update_default_template() {
        let conn = setup_test_db();
        let repo = PromptTemplateRepository::new(&conn);

        let template = PromptTemplate::default_template(
            "default_template",
            "Default Template",
            "Built-in template",
            "Default: {content}",
            vec![PromptVariable::required("content", "Content")],
            "analysis",
        );

        repo.create(&template).unwrap();

        // Try to update default template
        let result = repo.update(&template);
        assert!(result.is_err());
    }

    #[test]
    fn test_delete_template() {
        let conn = setup_test_db();
        let repo = PromptTemplateRepository::new(&conn);

        let template = PromptTemplate::new(
            "deletable_template",
            "Deletable Template",
            "Template for deletion",
            "Content: {content}",
            vec![PromptVariable::required("content", "Content")],
            "test",
        );

        repo.create(&template).unwrap();
        assert!(repo.delete("deletable_template").unwrap());

        let found = repo.find_by_id("deletable_template").unwrap();
        assert!(found.is_none());
    }

    #[test]
    fn test_cannot_delete_default_template() {
        let conn = setup_test_db();
        let repo = PromptTemplateRepository::new(&conn);

        let template = PromptTemplate::default_template(
            "default_template",
            "Default Template",
            "Built-in template",
            "Content: {content}",
            vec![PromptVariable::required("content", "Content")],
            "analysis",
        );

        repo.create(&template).unwrap();

        let result = repo.delete("default_template");
        assert!(result.is_err());
    }

    #[test]
    fn test_search_templates() {
        let conn = setup_test_db();
        let repo = PromptTemplateRepository::new(&conn);

        let template1 = PromptTemplate::new(
            "search_template1",
            "Analysis Template",
            "Template for analysis",
            "Analyze: {content}",
            vec![PromptVariable::required("content", "Content")],
            "analysis",
        );

        let template2 = PromptTemplate::new(
            "search_template2",
            "Report Template",
            "Template for reports",
            "Report: {content}",
            vec![PromptVariable::required("content", "Content")],
            "reporting",
        );

        repo.create(&template1).unwrap();
        repo.create(&template2).unwrap();

        let analysis_results = repo.search_templates("analysis").unwrap();
        assert_eq!(analysis_results.len(), 1);
        assert_eq!(analysis_results[0].id, "search_template1");

        let all_results = repo.search_templates("template").unwrap();
        assert_eq!(all_results.len(), 2);
    }

    #[test]
    fn test_clone_template() {
        let conn = setup_test_db();
        let repo = PromptTemplateRepository::new(&conn);

        let original = PromptTemplate::new(
            "original_template",
            "Original Template",
            "Original description",
            "Content: {content}",
            vec![PromptVariable::required("content", "Content")],
            "test",
        );

        repo.create(&original).unwrap();

        assert!(repo
            .clone_template("original_template", "cloned_template", "Cloned Template")
            .is_ok());

        let cloned = repo.find_by_id("cloned_template").unwrap().unwrap();
        assert_eq!(cloned.name, "Cloned Template");
        assert_eq!(cloned.template, original.template);
        assert_eq!(cloned.variables.len(), original.variables.len());
        assert!(!cloned.is_default);
    }

    #[test]
    fn test_category_operations() {
        let conn = setup_test_db();
        let repo = PromptTemplateRepository::new(&conn);

        let template1 = PromptTemplate::new(
            "cat1_template",
            "Category 1 Template",
            "Template in category 1",
            "Content: {content}",
            vec![PromptVariable::required("content", "Content")],
            "category1",
        );

        let template2 = PromptTemplate::new(
            "cat2_template",
            "Category 2 Template",
            "Template in category 2",
            "Content: {content}",
            vec![PromptVariable::required("content", "Content")],
            "category2",
        );

        repo.create(&template1).unwrap();
        repo.create(&template2).unwrap();

        let categories = repo.get_categories().unwrap();
        assert_eq!(categories.len(), 2);
        assert!(categories.contains(&"category1".to_string()));
        assert!(categories.contains(&"category2".to_string()));

        let cat1_templates = repo.find_by_category("category1").unwrap();
        assert_eq!(cat1_templates.len(), 1);
        assert_eq!(cat1_templates[0].id, "cat1_template");

        assert_eq!(repo.count_by_category("category1").unwrap(), 1);
        assert_eq!(repo.count_by_category("category2").unwrap(), 1);
        assert_eq!(repo.count_by_category("nonexistent").unwrap(), 0);
    }
}
