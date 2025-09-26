use crate::database::{DatabaseManager, PromptTemplateRepository};
use crate::models::{PromptTemplate, PromptVariable};
use anyhow::{anyhow, Result};
use std::collections::HashMap;
use tracing::{debug, info, warn};

#[derive(Debug, Clone)]
pub struct PromptService {
    db_manager: DatabaseManager,
}

impl PromptService {
    /// Create a new prompt service
    pub fn new(db_manager: DatabaseManager) -> Self {
        Self { db_manager }
    }

    /// Get all available templates
    pub fn list_templates(&self, include_defaults: bool) -> Result<Vec<PromptTemplate>> {
        self.db_manager.with_connection_anyhow(|conn| {
            let repo = PromptTemplateRepository::new(conn);
            repo.list_all(include_defaults)
        })
    }

    /// Get templates by category
    pub fn get_templates_by_category(&self, category: &str) -> Result<Vec<PromptTemplate>> {
        self.db_manager.with_connection_anyhow(|conn| {
            let repo = PromptTemplateRepository::new(conn);
            repo.find_by_category(category)
        })
    }

    /// Get a specific template by ID
    pub fn get_template(&self, template_id: &str) -> Result<Option<PromptTemplate>> {
        self.db_manager.with_connection_anyhow(|conn| {
            let repo = PromptTemplateRepository::new(conn);
            repo.find_by_id(template_id)
        })
    }

    /// Create a new custom template
    pub fn create_template(
        &self,
        id: &str,
        name: &str,
        description: &str,
        template: &str,
        variables: Vec<PromptVariable>,
        category: &str,
    ) -> Result<()> {
        // Validate template ID format
        if !Self::is_valid_template_id(id) {
            return Err(anyhow!(
                "Invalid template ID '{}'. Must contain only lowercase letters, numbers, hyphens, and underscores",
                id
            ));
        }

        let prompt_template =
            PromptTemplate::new(id, name, description, template, variables, category);

        // Validate template before creating
        prompt_template
            .validate()
            .map_err(|e| anyhow!("Validation failed: {}", e))?;

        self.db_manager.with_connection_anyhow(|conn| {
            let repo = PromptTemplateRepository::new(conn);

            // Check if template already exists
            if repo.exists(id)? {
                return Err(anyhow!("Template with ID '{}' already exists", id));
            }

            repo.create(&prompt_template)?;
            info!("Created custom template: {} ({})", name, id);
            Ok(())
        })
    }

    /// Update an existing custom template
    pub fn update_template(
        &self,
        template_id: &str,
        name: Option<String>,
        description: Option<String>,
        template: Option<String>,
        variables: Option<Vec<PromptVariable>>,
        category: Option<String>,
    ) -> Result<()> {
        self.db_manager.with_connection_anyhow(|conn| {
            let repo = PromptTemplateRepository::new(conn);

            let mut existing_template = repo
                .find_by_id(template_id)?
                .ok_or_else(|| anyhow!("Template '{}' not found", template_id))?;

            // Update the template
            existing_template
                .update(name, description, template, variables, category)
                .map_err(|e| anyhow!("Update validation failed: {}", e))?;

            repo.update(&existing_template)?;
            info!("Updated template: {}", template_id);
            Ok(())
        })
    }

    /// Delete a custom template
    pub fn delete_template(&self, template_id: &str) -> Result<()> {
        self.db_manager.with_connection_anyhow(|conn| {
            let repo = PromptTemplateRepository::new(conn);

            // Check if template is being used
            if !repo.validate_template_usage(template_id)? {
                return Err(anyhow!(
                    "Cannot delete template '{}' - it is currently being used in analyses or requests",
                    template_id
                ));
            }

            let deleted = repo.delete(template_id)?;
            if deleted {
                info!("Deleted template: {}", template_id);
                Ok(())
            } else {
                Err(anyhow!("Template '{}' not found or is a default template", template_id))
            }
        })
    }

    /// Clone an existing template with a new ID and name
    pub fn clone_template(&self, source_id: &str, new_id: &str, new_name: &str) -> Result<()> {
        if !Self::is_valid_template_id(new_id) {
            return Err(anyhow!(
                "Invalid template ID '{}'. Must contain only lowercase letters, numbers, hyphens, and underscores",
                new_id
            ));
        }

        self.db_manager.with_connection_anyhow(|conn| {
            let repo = PromptTemplateRepository::new(conn);
            repo.clone_template(source_id, new_id, new_name)?;
            info!(
                "Cloned template '{}' to '{}' ({})",
                source_id, new_id, new_name
            );
            Ok(())
        })
    }

    /// Render a template with provided variables
    pub fn render_template(
        &self,
        template_id: &str,
        variables: &HashMap<String, String>,
    ) -> Result<String> {
        let template = self
            .get_template(template_id)?
            .ok_or_else(|| anyhow!("Template '{}' not found", template_id))?;

        debug!(
            "Rendering template '{}' with {} variables",
            template_id,
            variables.len()
        );

        let rendered = template
            .render(variables)
            .map_err(|e| anyhow!("Rendering failed: {}", e))?;
        Ok(rendered)
    }

    /// Validate template variables against a template
    pub fn validate_template_variables(
        &self,
        template_id: &str,
        variables: &HashMap<String, String>,
    ) -> Result<ValidationResult> {
        let template = self
            .get_template(template_id)?
            .ok_or_else(|| anyhow!("Template '{}' not found", template_id))?;

        let mut result = ValidationResult {
            is_valid: true,
            missing_required: Vec::new(),
            unused_variables: Vec::new(),
            invalid_variables: Vec::new(),
        };

        // Check for missing required variables
        for var in template.get_required_variables() {
            if !variables.contains_key(&var.name) {
                result.missing_required.push(var.name.clone());
                result.is_valid = false;
            }
        }

        // Check for unused variables
        let template_var_names: std::collections::HashSet<String> =
            template.variables.iter().map(|v| v.name.clone()).collect();

        for var_name in variables.keys() {
            if !template_var_names.contains(var_name) {
                result.unused_variables.push(var_name.clone());
            }
        }

        // Check for empty values in required variables
        for var in template.get_required_variables() {
            if let Some(value) = variables.get(&var.name) {
                if value.trim().is_empty() {
                    result
                        .invalid_variables
                        .push(format!("Variable '{}' cannot be empty", var.name));
                    result.is_valid = false;
                }
            }
        }

        Ok(result)
    }

    /// Get all available categories
    pub fn get_categories(&self) -> Result<Vec<String>> {
        self.db_manager.with_connection_anyhow(|conn| {
            let repo = PromptTemplateRepository::new(conn);
            repo.get_categories()
        })
    }

    /// Search templates by name, description, or content
    pub fn search_templates(&self, search_term: &str) -> Result<Vec<PromptTemplate>> {
        if search_term.trim().is_empty() {
            return Ok(Vec::new());
        }

        self.db_manager.with_connection_anyhow(|conn| {
            let repo = PromptTemplateRepository::new(conn);
            repo.search_templates(search_term)
        })
    }

    /// Get template statistics
    pub fn get_template_statistics(&self) -> Result<TemplateStatistics> {
        let all_templates = self.db_manager.with_connection_anyhow(|conn| {
            let repo = PromptTemplateRepository::new(conn);
            repo.list_all(true)
        })?;

        let default_templates = self.db_manager.with_connection_anyhow(|conn| {
            let repo = PromptTemplateRepository::new(conn);
            repo.list_default_templates()
        })?;

        let categories = self.db_manager.with_connection_anyhow(|conn| {
            let repo = PromptTemplateRepository::new(conn);
            repo.get_categories()
        })?;

        let mut category_counts = HashMap::new();
        for template in &all_templates {
            *category_counts
                .entry(template.category.clone())
                .or_insert(0) += 1;
        }

        Ok(TemplateStatistics {
            total_templates: all_templates.len() as u32,
            default_templates: default_templates.len() as u32,
            custom_templates: (all_templates.len() - default_templates.len()) as u32,
            categories: categories.len() as u32,
            category_breakdown: category_counts.clone(),
            most_common_category: category_counts
                .iter()
                .max_by_key(|(_, count)| *count)
                .map(|(name, _)| name.clone()),
        })
    }

    /// Import templates from a configuration
    pub fn import_templates(&self, templates: Vec<TemplateConfig>) -> Result<ImportResult> {
        let mut result = ImportResult {
            imported: 0,
            skipped: 0,
            errors: Vec::new(),
        };

        for template_config in templates {
            match self.create_template(
                &template_config.id,
                &template_config.name,
                &template_config.description,
                &template_config.template,
                template_config.variables,
                &template_config.category,
            ) {
                Ok(()) => {
                    result.imported += 1;
                    info!(
                        "Imported template: {} ({})",
                        template_config.name, template_config.id
                    );
                }
                Err(e) if e.to_string().contains("already exists") => {
                    result.skipped += 1;
                    warn!("Skipped existing template: {}", template_config.id);
                }
                Err(e) => {
                    result
                        .errors
                        .push(format!("Failed to import '{}': {}", template_config.id, e));
                    warn!("Failed to import template '{}': {}", template_config.id, e);
                }
            }
        }

        info!(
            "Import completed: {} imported, {} skipped, {} errors",
            result.imported,
            result.skipped,
            result.errors.len()
        );

        Ok(result)
    }

    /// Export templates to a configuration format
    pub fn export_templates(&self, include_defaults: bool) -> Result<Vec<TemplateConfig>> {
        let templates = self.list_templates(include_defaults)?;

        let configs: Vec<TemplateConfig> = templates
            .into_iter()
            .map(|template| TemplateConfig {
                id: template.id,
                name: template.name,
                description: template.description,
                template: template.template,
                variables: template.variables,
                category: template.category,
            })
            .collect();

        info!("Exported {} templates", configs.len());
        Ok(configs)
    }

    /// Get a template with default variable values pre-filled
    pub fn get_template_with_defaults(
        &self,
        template_id: &str,
    ) -> Result<Option<RenderedTemplate>> {
        let template = match self.get_template(template_id)? {
            Some(t) => t,
            None => return Ok(None),
        };

        let mut variables = HashMap::new();

        // Fill in default values
        for var in &template.variables {
            if let Some(default) = &var.default_value {
                variables.insert(var.name.clone(), default.clone());
            }
        }

        let rendered_content = if variables.is_empty() {
            template.template.clone()
        } else {
            template
                .render(&variables)
                .unwrap_or(template.template.clone())
        };

        Ok(Some(RenderedTemplate {
            template: template.clone(),
            variables,
            rendered_content,
        }))
    }

    fn is_valid_template_id(id: &str) -> bool {
        !id.is_empty()
            && id.len() <= 64
            && id
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_')
    }
}

#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub is_valid: bool,
    pub missing_required: Vec<String>,
    pub unused_variables: Vec<String>,
    pub invalid_variables: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct TemplateStatistics {
    pub total_templates: u32,
    pub default_templates: u32,
    pub custom_templates: u32,
    pub categories: u32,
    pub category_breakdown: HashMap<String, u32>,
    pub most_common_category: Option<String>,
}

#[derive(Debug, Clone)]
pub struct TemplateConfig {
    pub id: String,
    pub name: String,
    pub description: String,
    pub template: String,
    pub variables: Vec<PromptVariable>,
    pub category: String,
}

#[derive(Debug, Clone)]
pub struct ImportResult {
    pub imported: u32,
    pub skipped: u32,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct RenderedTemplate {
    pub template: PromptTemplate,
    pub variables: HashMap<String, String>,
    pub rendered_content: String,
}

impl ValidationResult {
    pub fn has_errors(&self) -> bool {
        !self.missing_required.is_empty() || !self.invalid_variables.is_empty()
    }

    pub fn get_error_summary(&self) -> String {
        let mut errors = Vec::new();

        if !self.missing_required.is_empty() {
            errors.push(format!(
                "Missing required variables: {}",
                self.missing_required.join(", ")
            ));
        }

        if !self.invalid_variables.is_empty() {
            errors.push(format!(
                "Invalid variables: {}",
                self.invalid_variables.join(", ")
            ));
        }

        if !self.unused_variables.is_empty() {
            errors.push(format!(
                "Unused variables: {}",
                self.unused_variables.join(", ")
            ));
        }

        errors.join("; ")
    }
}

impl TemplateStatistics {
    pub fn get_summary(&self) -> String {
        format!(
            "{} total templates ({} default, {} custom) across {} categories",
            self.total_templates, self.default_templates, self.custom_templates, self.categories
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::DatabaseManager;
    use crate::models::PromptVariable;
    use tempfile::NamedTempFile;

    fn setup_test_service() -> PromptService {
        let temp_file = NamedTempFile::new().unwrap();
        let db_manager = DatabaseManager::new(temp_file.path().to_str().unwrap()).unwrap();

        // Initialize schema
        db_manager
            .with_connection(crate::database::create_schema)
            .unwrap();

        PromptService::new(db_manager)
    }

    #[test]
    fn test_template_id_validation() {
        assert!(PromptService::is_valid_template_id("valid-template_123"));
        assert!(PromptService::is_valid_template_id("simple"));
        assert!(PromptService::is_valid_template_id("test_template"));
        assert!(PromptService::is_valid_template_id("template-with-dashes"));

        assert!(!PromptService::is_valid_template_id(""));
        assert!(!PromptService::is_valid_template_id("Invalid Template"));
        assert!(!PromptService::is_valid_template_id("template.with.dots"));
        assert!(!PromptService::is_valid_template_id("UPPERCASE"));
        assert!(!PromptService::is_valid_template_id(&"a".repeat(70)));
    }

    #[test]
    fn test_create_and_get_template() {
        let service = setup_test_service();

        let variables = vec![
            PromptVariable::required("content", "Content to analyze"),
            PromptVariable::optional("focus", "Analysis focus", "general"),
        ];

        // Create template
        let result = service.create_template(
            "test-template",
            "Test Template",
            "A template for testing",
            "Analyze: {content} with focus on {focus}",
            variables,
            "test",
        );
        assert!(result.is_ok());

        // Get template
        let template = service.get_template("test-template").unwrap();
        assert!(template.is_some());

        let template = template.unwrap();
        assert_eq!(template.id, "test-template");
        assert_eq!(template.name, "Test Template");
        assert_eq!(template.variables.len(), 2);
    }

    #[test]
    fn test_template_rendering() {
        let service = setup_test_service();

        let variables = vec![
            PromptVariable::required("name", "Name"),
            PromptVariable::optional("greeting", "Greeting", "Hello"),
        ];

        service
            .create_template(
                "greeting-template",
                "Greeting Template",
                "Template for greetings",
                "{greeting}, {name}!",
                variables,
                "test",
            )
            .unwrap();

        // Render with all variables
        let mut vars = HashMap::new();
        vars.insert("name".to_string(), "World".to_string());
        vars.insert("greeting".to_string(), "Hi".to_string());

        let rendered = service.render_template("greeting-template", &vars).unwrap();
        assert_eq!(rendered, "Hi, World!");

        // Render with only required variable (should use default)
        let mut vars = HashMap::new();
        vars.insert("name".to_string(), "Alice".to_string());

        let rendered = service.render_template("greeting-template", &vars).unwrap();
        assert_eq!(rendered, "Hello, Alice!");
    }

    #[test]
    fn test_variable_validation() {
        let service = setup_test_service();

        let variables = vec![
            PromptVariable::required("required_var", "Required variable"),
            PromptVariable::optional("optional_var", "Optional variable", "default"),
        ];

        service
            .create_template(
                "validation-template",
                "Validation Template",
                "Template for validation testing",
                "Required: {required_var}, Optional: {optional_var}",
                variables,
                "test",
            )
            .unwrap();

        // Valid variables
        let mut vars = HashMap::new();
        vars.insert("required_var".to_string(), "value".to_string());

        let result = service
            .validate_template_variables("validation-template", &vars)
            .unwrap();
        assert!(result.is_valid);
        assert!(result.missing_required.is_empty());

        // Missing required variable
        let vars = HashMap::new();
        let result = service
            .validate_template_variables("validation-template", &vars)
            .unwrap();
        assert!(!result.is_valid);
        assert_eq!(result.missing_required, vec!["required_var"]);

        // Unused variables
        let mut vars = HashMap::new();
        vars.insert("required_var".to_string(), "value".to_string());
        vars.insert("unused_var".to_string(), "unused".to_string());

        let result = service
            .validate_template_variables("validation-template", &vars)
            .unwrap();
        assert!(result.is_valid); // Still valid, just has unused vars
        assert_eq!(result.unused_variables, vec!["unused_var"]);
    }

    #[test]
    fn test_template_cloning() {
        let service = setup_test_service();

        // Create original template
        let variables = vec![PromptVariable::required("content", "Content")];
        service
            .create_template(
                "original",
                "Original Template",
                "Original description",
                "Content: {content}",
                variables,
                "test",
            )
            .unwrap();

        // Clone template
        let result = service.clone_template("original", "cloned", "Cloned Template");
        assert!(result.is_ok());

        // Verify clone exists
        let cloned = service.get_template("cloned").unwrap();
        assert!(cloned.is_some());

        let cloned = cloned.unwrap();
        assert_eq!(cloned.name, "Cloned Template");
        assert_eq!(cloned.template, "Content: {content}");
        assert!(!cloned.is_default);
    }

    #[test]
    fn test_template_search() {
        let service = setup_test_service();

        // Create test templates
        service
            .create_template(
                "analysis-template",
                "Analysis Template",
                "For analyzing data",
                "Analyze: {data}",
                vec![PromptVariable::required("data", "Data")],
                "analysis",
            )
            .unwrap();

        service
            .create_template(
                "report-template",
                "Report Template",
                "For generating reports",
                "Report: {content}",
                vec![PromptVariable::required("content", "Content")],
                "reporting",
            )
            .unwrap();

        // Search by name
        let results = service.search_templates("analysis").unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "analysis-template");

        // Search by description
        let results = service.search_templates("reports").unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "report-template");

        // Search by template content
        let results = service.search_templates("Analyze").unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "analysis-template");
    }

    #[test]
    fn test_template_statistics() {
        let service = setup_test_service();

        // Create test templates in different categories
        service
            .create_template(
                "analysis1",
                "Analysis 1",
                "First analysis template",
                "Content: {content}",
                vec![PromptVariable::required("content", "Content")],
                "analysis",
            )
            .unwrap();

        service
            .create_template(
                "analysis2",
                "Analysis 2",
                "Second analysis template",
                "Content: {content}",
                vec![PromptVariable::required("content", "Content")],
                "analysis",
            )
            .unwrap();

        service
            .create_template(
                "report1",
                "Report 1",
                "Report template",
                "Content: {content}",
                vec![PromptVariable::required("content", "Content")],
                "reporting",
            )
            .unwrap();

        let stats = service.get_template_statistics().unwrap();
        assert_eq!(stats.total_templates, 3);
        assert_eq!(stats.custom_templates, 3);
        assert_eq!(stats.categories, 2);
        assert_eq!(stats.most_common_category, Some("analysis".to_string()));
    }
}
