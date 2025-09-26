use chrono::{DateTime, Utc};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// Defines variables within prompt templates
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PromptVariable {
    /// Variable name (matches template placeholder)
    pub name: String,
    /// Variable purpose and expected content
    pub description: String,
    /// Whether variable must be provided
    pub required: bool,
    /// Default value if optional
    pub default_value: Option<String>,
}

impl PromptVariable {
    /// Create a new required variable
    pub fn required(name: &str, description: &str) -> Self {
        Self {
            name: name.to_string(),
            description: description.to_string(),
            required: true,
            default_value: None,
        }
    }

    /// Create a new optional variable with default value
    pub fn optional(name: &str, description: &str, default: &str) -> Self {
        Self {
            name: name.to_string(),
            description: description.to_string(),
            required: false,
            default_value: Some(default.to_string()),
        }
    }

    /// Validate variable name follows naming conventions
    pub fn validate_name(&self) -> Result<(), String> {
        let name_pattern =
            Regex::new(r"^[a-zA-Z_][a-zA-Z0-9_]*$").map_err(|e| format!("Regex error: {}", e))?;

        if !name_pattern.is_match(&self.name) {
            return Err(format!(
                "Variable name '{}' must start with letter or underscore and contain only alphanumeric characters and underscores",
                self.name
            ));
        }

        if self.name.len() > 64 {
            return Err(format!(
                "Variable name '{}' exceeds maximum length of 64 characters",
                self.name
            ));
        }

        Ok(())
    }

    /// Check if this variable has a default value
    pub fn has_default(&self) -> bool {
        self.default_value.is_some()
    }
}

/// Stores configurable analysis prompt templates
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptTemplate {
    /// Unique template identifier (slug format)
    pub id: String,
    /// Human-readable template name
    pub name: String,
    /// Template purpose and usage description
    pub description: String,
    /// Prompt template with variable placeholders
    pub template: String,
    /// Template variable definitions
    pub variables: Vec<PromptVariable>,
    /// Template category (analysis, retrospective, custom)
    pub category: String,
    /// Whether this is a built-in template
    pub is_default: bool,
    /// Template creation time
    pub created_at: DateTime<Utc>,
    /// Last modification time
    pub modified_at: DateTime<Utc>,
}

impl PromptTemplate {
    /// Create a new custom template
    pub fn new(
        id: &str,
        name: &str,
        description: &str,
        template: &str,
        variables: Vec<PromptVariable>,
        category: &str,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: id.to_string(),
            name: name.to_string(),
            description: description.to_string(),
            template: template.to_string(),
            variables,
            category: category.to_string(),
            is_default: false,
            created_at: now,
            modified_at: now,
        }
    }

    /// Create a default template (built-in)
    pub fn default_template(
        id: &str,
        name: &str,
        description: &str,
        template: &str,
        variables: Vec<PromptVariable>,
        category: &str,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: id.to_string(),
            name: name.to_string(),
            description: description.to_string(),
            template: template.to_string(),
            variables,
            category: category.to_string(),
            is_default: true,
            created_at: now,
            modified_at: now,
        }
    }

    /// Validate the template for consistency and correctness
    pub fn validate(&self) -> Result<(), String> {
        // Validate ID format (URL-safe)
        self.validate_id()?;

        // Validate template size
        if self.template.len() > 8192 {
            return Err("Template exceeds maximum size of 8192 characters".to_string());
        }

        if self.template.is_empty() {
            return Err("Template content cannot be empty".to_string());
        }

        // Validate variable names
        for variable in &self.variables {
            variable.validate_name()?;
        }

        // Check for duplicate variable names
        let mut variable_names = HashSet::new();
        for variable in &self.variables {
            if !variable_names.insert(&variable.name) {
                return Err(format!("Duplicate variable name: {}", variable.name));
            }
        }

        // Validate that all template placeholders have corresponding variables
        let template_vars = self.extract_template_variables()?;
        let defined_vars: HashSet<&String> = self.variables.iter().map(|v| &v.name).collect();

        for template_var in &template_vars {
            if !defined_vars.contains(template_var) {
                return Err(format!(
                    "Template references undefined variable: {}",
                    template_var
                ));
            }
        }

        // Validate that all defined variables are used in template
        for variable in &self.variables {
            if !template_vars.contains(&variable.name) {
                return Err(format!(
                    "Variable '{}' is defined but not used in template",
                    variable.name
                ));
            }
        }

        // Check for circular references in default values
        self.validate_no_circular_references()?;

        Ok(())
    }

    /// Validate template ID format
    fn validate_id(&self) -> Result<(), String> {
        let id_pattern = Regex::new(r"^[a-z0-9_-]+$").map_err(|e| format!("Regex error: {}", e))?;

        if !id_pattern.is_match(&self.id) {
            return Err(format!(
                "Template ID '{}' must contain only lowercase letters, numbers, hyphens, and underscores",
                self.id
            ));
        }

        if self.id.len() > 64 {
            return Err(format!(
                "Template ID '{}' exceeds maximum length of 64 characters",
                self.id
            ));
        }

        Ok(())
    }

    /// Extract variable placeholders from template text
    fn extract_template_variables(&self) -> Result<HashSet<String>, String> {
        let var_pattern = Regex::new(r"\{([a-zA-Z_][a-zA-Z0-9_]*)\}")
            .map_err(|e| format!("Regex error: {}", e))?;

        let variables: HashSet<String> = var_pattern
            .captures_iter(&self.template)
            .map(|cap| cap[1].to_string())
            .collect();

        Ok(variables)
    }

    /// Check for circular references in default values
    fn validate_no_circular_references(&self) -> Result<(), String> {
        for variable in &self.variables {
            if let Some(default_value) = &variable.default_value {
                if self.has_circular_reference(
                    &variable.name,
                    default_value,
                    &mut HashSet::new(),
                )? {
                    return Err(format!(
                        "Circular reference detected in variable '{}'",
                        variable.name
                    ));
                }
            }
        }
        Ok(())
    }

    /// Recursively check for circular references
    fn has_circular_reference(
        &self,
        var_name: &str,
        value: &str,
        visited: &mut HashSet<String>,
    ) -> Result<bool, String> {
        if visited.contains(var_name) {
            return Ok(true);
        }

        visited.insert(var_name.to_string());

        let referenced_vars = self.extract_variables_from_text(value)?;
        for referenced_var in referenced_vars {
            if let Some(variable) = self.variables.iter().find(|v| v.name == referenced_var) {
                if let Some(default_value) = &variable.default_value {
                    if self.has_circular_reference(&referenced_var, default_value, visited)? {
                        return Ok(true);
                    }
                }
            }
        }

        visited.remove(var_name);
        Ok(false)
    }

    /// Extract variable references from arbitrary text
    fn extract_variables_from_text(&self, text: &str) -> Result<Vec<String>, String> {
        let var_pattern = Regex::new(r"\{([a-zA-Z_][a-zA-Z0-9_]*)\}")
            .map_err(|e| format!("Regex error: {}", e))?;

        let variables: Vec<String> = var_pattern
            .captures_iter(text)
            .map(|cap| cap[1].to_string())
            .collect();

        Ok(variables)
    }

    /// Render the template with provided variable values
    pub fn render(&self, variables: &HashMap<String, String>) -> Result<String, String> {
        let mut rendered = self.template.clone();

        // Check that all required variables are provided
        for variable in &self.variables {
            if variable.required && !variables.contains_key(&variable.name) {
                return Err(format!(
                    "Required variable '{}' not provided",
                    variable.name
                ));
            }
        }

        // Replace variables with their values
        for variable in &self.variables {
            let value = if let Some(provided_value) = variables.get(&variable.name) {
                provided_value.clone()
            } else if let Some(default_value) = &variable.default_value {
                default_value.clone()
            } else {
                continue; // Should not happen due to required check above
            };

            let placeholder = format!("{{{}}}", variable.name);
            rendered = rendered.replace(&placeholder, &value);
        }

        // Check for any remaining unreplaced placeholders
        let remaining_vars = self.extract_variables_from_text(&rendered)?;
        if !remaining_vars.is_empty() {
            return Err(format!(
                "Template contains unreplaced variables: {}",
                remaining_vars.join(", ")
            ));
        }

        Ok(rendered)
    }

    /// Get required variables
    pub fn get_required_variables(&self) -> Vec<&PromptVariable> {
        self.variables.iter().filter(|v| v.required).collect()
    }

    /// Get optional variables
    pub fn get_optional_variables(&self) -> Vec<&PromptVariable> {
        self.variables.iter().filter(|v| !v.required).collect()
    }

    /// Check if template can be modified (not a default template)
    pub fn is_modifiable(&self) -> bool {
        !self.is_default
    }

    /// Update the template (only for non-default templates)
    pub fn update(
        &mut self,
        name: Option<String>,
        description: Option<String>,
        template: Option<String>,
        variables: Option<Vec<PromptVariable>>,
        category: Option<String>,
    ) -> Result<(), String> {
        if self.is_default {
            return Err("Cannot modify default template".to_string());
        }

        if let Some(name) = name {
            self.name = name;
        }
        if let Some(description) = description {
            self.description = description;
        }
        if let Some(template) = template {
            self.template = template;
        }
        if let Some(variables) = variables {
            self.variables = variables;
        }
        if let Some(category) = category {
            self.category = category;
        }

        self.modified_at = Utc::now();

        // Validate after update
        self.validate()?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prompt_variable_validation() {
        let valid_var = PromptVariable::required("chat_content", "Chat content to analyze");
        assert!(valid_var.validate_name().is_ok());

        let invalid_var = PromptVariable::required("123invalid", "Invalid name");
        assert!(invalid_var.validate_name().is_err());

        let long_name_var = PromptVariable::required(&"a".repeat(70), "Too long");
        assert!(long_name_var.validate_name().is_err());
    }

    #[test]
    fn test_template_creation() {
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

        assert_eq!(template.id, "test_template");
        assert!(!template.is_default);
        assert_eq!(template.variables.len(), 2);
    }

    #[test]
    fn test_template_validation() {
        let variables = vec![PromptVariable::required("chat_content", "Chat content")];

        let valid_template = PromptTemplate::new(
            "valid_template",
            "Valid Template",
            "Valid template",
            "Content: {chat_content}",
            variables,
            "test",
        );

        assert!(valid_template.validate().is_ok());

        // Test template with undefined variable
        let invalid_template = PromptTemplate::new(
            "invalid_template",
            "Invalid Template",
            "Invalid template",
            "Content: {undefined_var}",
            vec![],
            "test",
        );

        assert!(invalid_template.validate().is_err());
    }

    #[test]
    fn test_template_rendering() {
        let variables = vec![
            PromptVariable::required("name", "Name"),
            PromptVariable::optional("greeting", "Greeting", "Hello"),
        ];

        let template = PromptTemplate::new(
            "greeting_template",
            "Greeting Template",
            "Template for greetings",
            "{greeting}, {name}!",
            variables,
            "test",
        );

        let mut vars = HashMap::new();
        vars.insert("name".to_string(), "World".to_string());

        let rendered = template.render(&vars).unwrap();
        assert_eq!(rendered, "Hello, World!");

        // Test with custom greeting
        vars.insert("greeting".to_string(), "Hi".to_string());
        let rendered = template.render(&vars).unwrap();
        assert_eq!(rendered, "Hi, World!");
    }

    #[test]
    fn test_circular_reference_detection() {
        let variables = vec![
            PromptVariable::optional("var_a", "Variable A", "Value of {var_b}"),
            PromptVariable::optional("var_b", "Variable B", "Value of {var_a}"),
        ];

        let template = PromptTemplate::new(
            "circular_template",
            "Circular Template",
            "Template with circular references",
            "A: {var_a}, B: {var_b}",
            variables,
            "test",
        );

        assert!(template.validate().is_err());
    }

    #[test]
    fn test_template_modification() {
        let mut template = PromptTemplate::new(
            "modifiable_template",
            "Original Name",
            "Original description",
            "Original: {content}",
            vec![PromptVariable::required("content", "Content")],
            "test",
        );

        assert!(template.is_modifiable());

        let result = template.update(
            Some("Updated Name".to_string()),
            Some("Updated description".to_string()),
            None,
            None,
            None,
        );

        assert!(result.is_ok());
        assert_eq!(template.name, "Updated Name");
        assert_eq!(template.description, "Updated description");

        // Test that default templates cannot be modified
        let mut default_template = PromptTemplate::default_template(
            "default_template",
            "Default Template",
            "Built-in template",
            "Default: {content}",
            vec![PromptVariable::required("content", "Content")],
            "analysis",
        );

        assert!(!default_template.is_modifiable());
        let result = default_template.update(Some("New Name".to_string()), None, None, None, None);
        assert!(result.is_err());
    }

    #[test]
    fn test_variable_helpers() {
        let variables = vec![
            PromptVariable::required("required_var", "Required"),
            PromptVariable::optional("optional_var", "Optional", "default"),
        ];

        let template = PromptTemplate::new(
            "helper_test",
            "Helper Test",
            "Testing helper methods",
            "Required: {required_var}, Optional: {optional_var}",
            variables,
            "test",
        );

        let required = template.get_required_variables();
        let optional = template.get_optional_variables();

        assert_eq!(required.len(), 1);
        assert_eq!(optional.len(), 1);
        assert_eq!(required[0].name, "required_var");
        assert_eq!(optional[0].name, "optional_var");
        assert!(optional[0].has_default());
        assert!(!required[0].has_default());
    }
}
