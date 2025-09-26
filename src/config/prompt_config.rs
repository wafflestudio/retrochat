use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

use crate::models::prompt_template::{PromptTemplate, PromptVariable};

/// Configuration structure for prompt templates
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptConfig {
    /// Template metadata
    pub metadata: TemplateMetadata,
    /// Collection of prompt templates
    pub templates: HashMap<String, TemplateConfig>,
}

/// Metadata about the template collection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateMetadata {
    /// Version of the template format
    pub version: String,
    /// Description of this template collection
    pub description: String,
    /// Author or source of templates
    pub author: Option<String>,
    /// Creation or last update timestamp
    pub updated_at: String,
}

/// Configuration for a single prompt template
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateConfig {
    /// Human-readable template name
    pub name: String,
    /// Template purpose and usage description
    pub description: String,
    /// Template category (analysis, technical, learning, custom)
    pub category: String,
    /// Whether this template is active/enabled
    pub enabled: bool,
    /// Template variable definitions
    pub variables: Vec<VariableConfig>,
    /// The actual prompt template content
    pub template: String,
}

/// Configuration for template variables
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VariableConfig {
    /// Variable name (must match template placeholder)
    pub name: String,
    /// Variable description
    pub description: String,
    /// Whether the variable is required
    pub required: bool,
    /// Default value if not required
    pub default_value: Option<String>,
}

impl PromptConfig {
    /// Create a new empty prompt configuration
    pub fn new() -> Self {
        Self {
            metadata: TemplateMetadata {
                version: "1.0.0".to_string(),
                description: "RetroChat prompt templates".to_string(),
                author: None,
                updated_at: chrono::Utc::now().to_rfc3339(),
            },
            templates: HashMap::new(),
        }
    }

    /// Create a configuration from a collection of prompt templates
    pub fn from_templates(templates: Vec<PromptTemplate>, description: &str) -> Self {
        let mut config = Self::new();
        config.metadata.description = description.to_string();

        for template in templates {
            let template_config = TemplateConfig {
                name: template.name,
                description: template.description,
                category: template.category,
                enabled: !template.is_default, // Default templates start enabled
                variables: template
                    .variables
                    .into_iter()
                    .map(|v| VariableConfig {
                        name: v.name,
                        description: v.description,
                        required: v.required,
                        default_value: v.default_value,
                    })
                    .collect(),
                template: template.template,
            };

            config.templates.insert(template.id, template_config);
        }

        config
    }

    /// Convert configuration back to prompt templates
    pub fn to_templates(&self) -> Vec<PromptTemplate> {
        self.templates
            .iter()
            .filter(|(_, config)| config.enabled)
            .map(|(id, config)| {
                let variables = config
                    .variables
                    .iter()
                    .map(|v| PromptVariable {
                        name: v.name.clone(),
                        description: v.description.clone(),
                        required: v.required,
                        default_value: v.default_value.clone(),
                    })
                    .collect();

                PromptTemplate::new(
                    id,
                    &config.name,
                    &config.description,
                    &config.template,
                    variables,
                    &config.category,
                )
            })
            .collect()
    }

    /// Load configuration from TOML file
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let contents = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read config file: {}", path.as_ref().display()))?;

        toml::from_str(&contents)
            .with_context(|| format!("Failed to parse config file: {}", path.as_ref().display()))
    }

    /// Save configuration to TOML file
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let contents =
            toml::to_string_pretty(self).context("Failed to serialize configuration to TOML")?;

        // Ensure parent directory exists
        if let Some(parent) = path.as_ref().parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create directory: {}", parent.display()))?;
        }

        fs::write(&path, contents)
            .with_context(|| format!("Failed to write config file: {}", path.as_ref().display()))?;

        Ok(())
    }

    /// Add a new template to the configuration
    pub fn add_template(&mut self, id: String, template: TemplateConfig) {
        self.templates.insert(id, template);
        self.metadata.updated_at = chrono::Utc::now().to_rfc3339();
    }

    /// Remove a template from the configuration
    pub fn remove_template(&mut self, id: &str) -> Option<TemplateConfig> {
        let result = self.templates.remove(id);
        if result.is_some() {
            self.metadata.updated_at = chrono::Utc::now().to_rfc3339();
        }
        result
    }

    /// Update an existing template in the configuration
    pub fn update_template(&mut self, id: &str, template: TemplateConfig) -> bool {
        if self.templates.contains_key(id) {
            self.templates.insert(id.to_string(), template);
            self.metadata.updated_at = chrono::Utc::now().to_rfc3339();
            true
        } else {
            false
        }
    }

    /// Get a template by ID
    pub fn get_template(&self, id: &str) -> Option<&TemplateConfig> {
        self.templates.get(id)
    }

    /// Get all enabled template IDs
    pub fn get_enabled_template_ids(&self) -> Vec<String> {
        self.templates
            .iter()
            .filter(|(_, config)| config.enabled)
            .map(|(id, _)| id.clone())
            .collect()
    }

    /// Get templates by category
    pub fn get_templates_by_category(&self, category: &str) -> HashMap<String, &TemplateConfig> {
        self.templates
            .iter()
            .filter(|(_, config)| config.category == category && config.enabled)
            .map(|(id, config)| (id.clone(), config))
            .collect()
    }

    /// Validate all templates in the configuration
    pub fn validate(&self) -> Result<()> {
        for (id, template) in &self.templates {
            // Check that template contains all required variable placeholders
            for variable in &template.variables {
                if variable.required {
                    let placeholder = format!("{{{}}}", variable.name);
                    if !template.template.contains(&placeholder) {
                        return Err(anyhow::anyhow!(
                            "Template '{}' is missing required variable placeholder '{}'",
                            id,
                            placeholder
                        ));
                    }
                }
            }

            // Check that all placeholders have corresponding variables
            let placeholders = extract_placeholders(&template.template);
            for placeholder in placeholders {
                if !template.variables.iter().any(|v| v.name == placeholder) {
                    return Err(anyhow::anyhow!(
                        "Template '{}' has placeholder '{}' without corresponding variable definition",
                        id,
                        placeholder
                    ));
                }
            }
        }

        Ok(())
    }

    /// Get statistics about the configuration
    pub fn get_statistics(&self) -> ConfigStatistics {
        let total_templates = self.templates.len();
        let enabled_templates = self.templates.values().filter(|t| t.enabled).count();
        let disabled_templates = total_templates - enabled_templates;

        let mut categories = HashMap::new();
        for template in self.templates.values() {
            *categories.entry(template.category.clone()).or_insert(0) += 1;
        }

        ConfigStatistics {
            total_templates,
            enabled_templates,
            disabled_templates,
            categories,
        }
    }
}

/// Statistics about a prompt configuration
#[derive(Debug, Clone)]
pub struct ConfigStatistics {
    pub total_templates: usize,
    pub enabled_templates: usize,
    pub disabled_templates: usize,
    pub categories: HashMap<String, usize>,
}

impl Default for PromptConfig {
    fn default() -> Self {
        Self::new()
    }
}

/// Extract placeholder variable names from a template string
fn extract_placeholders(template: &str) -> Vec<String> {
    let mut placeholders = Vec::new();
    let mut chars = template.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '{' {
            let mut placeholder = String::new();
            while let Some(&next_ch) = chars.peek() {
                if next_ch == '}' {
                    chars.next(); // consume the '}'
                    break;
                }
                placeholder.push(chars.next().unwrap());
            }
            if !placeholder.is_empty() {
                placeholders.push(placeholder);
            }
        }
    }

    placeholders
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_prompt_config_creation() {
        let config = PromptConfig::new();
        assert_eq!(config.metadata.version, "1.0.0");
        assert!(config.templates.is_empty());
    }

    #[test]
    fn test_extract_placeholders() {
        let template = "Hello {name}, your {item} is ready. Total: {price}";
        let placeholders = extract_placeholders(template);
        assert_eq!(placeholders, vec!["name", "item", "price"]);
    }

    #[test]
    fn test_extract_placeholders_nested_braces() {
        let template = "Complex {outer_{inner}_test} and {simple}";
        let placeholders = extract_placeholders(template);
        // Should handle nested braces gracefully
        assert!(placeholders.contains(&"simple".to_string()));
    }

    #[test]
    fn test_config_file_operations() {
        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join("test_config.toml");

        let mut config = PromptConfig::new();
        config.add_template(
            "test".to_string(),
            TemplateConfig {
                name: "Test Template".to_string(),
                description: "A test template".to_string(),
                category: "test".to_string(),
                enabled: true,
                variables: vec![VariableConfig {
                    name: "content".to_string(),
                    description: "Test content".to_string(),
                    required: true,
                    default_value: None,
                }],
                template: "Test: {content}".to_string(),
            },
        );

        // Save and load
        config.save_to_file(&config_path).unwrap();
        let loaded_config = PromptConfig::load_from_file(&config_path).unwrap();

        assert_eq!(loaded_config.templates.len(), 1);
        assert!(loaded_config.templates.contains_key("test"));
    }

    #[test]
    fn test_template_validation() {
        let mut config = PromptConfig::new();

        // Valid template
        config.add_template(
            "valid".to_string(),
            TemplateConfig {
                name: "Valid".to_string(),
                description: "Valid template".to_string(),
                category: "test".to_string(),
                enabled: true,
                variables: vec![VariableConfig {
                    name: "content".to_string(),
                    description: "Content".to_string(),
                    required: true,
                    default_value: None,
                }],
                template: "Content: {content}".to_string(),
            },
        );

        assert!(config.validate().is_ok());

        // Invalid template - missing variable placeholder
        config.add_template(
            "invalid".to_string(),
            TemplateConfig {
                name: "Invalid".to_string(),
                description: "Invalid template".to_string(),
                category: "test".to_string(),
                enabled: true,
                variables: vec![VariableConfig {
                    name: "missing".to_string(),
                    description: "Missing placeholder".to_string(),
                    required: true,
                    default_value: None,
                }],
                template: "No placeholder here".to_string(),
            },
        );

        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_statistics() {
        let mut config = PromptConfig::new();

        config.add_template(
            "t1".to_string(),
            TemplateConfig {
                name: "T1".to_string(),
                description: "Test 1".to_string(),
                category: "analysis".to_string(),
                enabled: true,
                variables: vec![],
                template: "Template 1".to_string(),
            },
        );

        config.add_template(
            "t2".to_string(),
            TemplateConfig {
                name: "T2".to_string(),
                description: "Test 2".to_string(),
                category: "analysis".to_string(),
                enabled: false,
                variables: vec![],
                template: "Template 2".to_string(),
            },
        );

        config.add_template(
            "t3".to_string(),
            TemplateConfig {
                name: "T3".to_string(),
                description: "Test 3".to_string(),
                category: "technical".to_string(),
                enabled: true,
                variables: vec![],
                template: "Template 3".to_string(),
            },
        );

        let stats = config.get_statistics();
        assert_eq!(stats.total_templates, 3);
        assert_eq!(stats.enabled_templates, 2);
        assert_eq!(stats.disabled_templates, 1);
        assert_eq!(stats.categories.get("analysis"), Some(&2));
        assert_eq!(stats.categories.get("technical"), Some(&1));
    }
}
