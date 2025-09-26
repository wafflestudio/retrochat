use anyhow::{anyhow, Result};
use std::env;

pub mod defaults;
pub mod directories;
pub mod prompt_config;

pub use defaults::get_default_templates;
pub use directories::AppDirectories;
pub use prompt_config::{ConfigStatistics, PromptConfig, TemplateConfig, VariableConfig};

/// Validates required environment variables for retrospection features
pub fn validate_environment() -> Result<()> {
    if env::var("GEMINI_API_KEY").is_err() {
        return Err(anyhow!(
            "GEMINI_API_KEY environment variable is required for retrospection analysis. \
             Please set it with: export GEMINI_API_KEY=\"your-api-key\""
        ));
    }
    Ok(())
}

/// Gets the Gemini API key from environment
pub fn get_gemini_api_key() -> Result<String> {
    env::var("GEMINI_API_KEY").map_err(|_| {
        anyhow!(
            "GEMINI_API_KEY environment variable not found. \
             Please set it with: export GEMINI_API_KEY=\"your-api-key\""
        )
    })
}

/// Initialize application configuration and directories
pub fn initialize_app_config() -> Result<AppDirectories> {
    let app_dirs = AppDirectories::new()?;

    // Ensure all necessary directories exist
    app_dirs.ensure_directories()?;
    app_dirs.ensure_additional_directories()?;

    // Initialize default templates if they don't exist
    let default_templates_path = app_dirs.default_templates_path();
    if !default_templates_path.exists() {
        let default_templates = get_default_templates();
        let config = PromptConfig::from_templates(
            default_templates,
            "Default RetroChat prompt templates for retrospection analysis",
        );
        config.save_to_file(&default_templates_path)?;
    }

    Ok(app_dirs)
}

/// Load all available prompt templates from configuration
pub fn load_all_templates(
    app_dirs: &AppDirectories,
) -> Result<Vec<crate::models::prompt_template::PromptTemplate>> {
    let mut all_templates = Vec::new();

    // Load default templates
    let default_templates_path = app_dirs.default_templates_path();
    if default_templates_path.exists() {
        let default_config = PromptConfig::load_from_file(&default_templates_path)?;
        all_templates.extend(default_config.to_templates());
    }

    // Load custom templates if they exist
    let custom_templates_path = app_dirs.custom_templates_path();
    if custom_templates_path.exists() {
        let custom_config = PromptConfig::load_from_file(&custom_templates_path)?;
        all_templates.extend(custom_config.to_templates());
    }

    Ok(all_templates)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_validate_environment_missing_key() {
        env::remove_var("GEMINI_API_KEY");
        assert!(validate_environment().is_err());
    }

    #[test]
    fn test_validate_environment_with_key() {
        env::set_var("GEMINI_API_KEY", "test-key");
        assert!(validate_environment().is_ok());
        env::remove_var("GEMINI_API_KEY");
    }

    #[test]
    fn test_get_gemini_api_key() {
        env::set_var("GEMINI_API_KEY", "test-key-123");
        assert_eq!(get_gemini_api_key().unwrap(), "test-key-123");
        env::remove_var("GEMINI_API_KEY");
    }
}
