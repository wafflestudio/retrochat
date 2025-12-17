use super::base::ProviderConfig;
use crate::env::providers as env_vars;
use crate::models::provider::ParserType;
use anyhow::Result;
use std::path::Path;

pub struct GeminiCliConfig;

impl GeminiCliConfig {
    pub fn create() -> ProviderConfig {
        ProviderConfig::new("Gemini CLI".to_string(), ParserType::GeminiJson)
            .with_cli_name("gemini".to_string())
            .with_description("Gemini CLI (.json files)".to_string())
            .with_env_var_name(env_vars::GEMINI_DIRS.to_string())
            .with_default_directory("~/.gemini/tmp".to_string())
            .with_file_patterns(vec!["session-*.json".to_string()])
            .with_default_location("darwin".to_string(), vec!["~/.gemini/tmp".to_string()])
            .with_default_location("linux".to_string(), vec!["~/.gemini/tmp".to_string()])
            .with_default_location(
                "windows".to_string(),
                vec!["%APPDATA%/Gemini/tmp".to_string()],
            )
            .with_token_support()
    }

    pub async fn import_directories<F>(overwrite: bool, import_batch_fn: F) -> Result<()>
    where
        F: Fn(
            String,
            bool,
        ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<()>> + Send>>,
    {
        let config = Self::create();
        let directories = config.get_import_directories();

        if directories.is_empty() {
            println!(
                "  No Gemini directories configured. Set {} environment variable.",
                env_vars::GEMINI_DIRS
            );
            return Ok(());
        }

        let mut imported_any = false;

        for dir_path in directories {
            let path = Path::new(&dir_path);
            if path.exists() {
                println!("  Importing from: {}", path.display());
                if let Err(e) = import_batch_fn(dir_path, overwrite).await {
                    eprintln!("  Error: {e}");
                } else {
                    imported_any = true;
                }
            } else {
                println!("  Directory not found: {}", path.display());
            }
        }

        if !imported_any {
            println!("  No Gemini directories found or imported");
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_gemini_cli_config() {
        let config = GeminiCliConfig::create();

        // Check CLI name exists and is correct
        assert_eq!(config.cli_name(), "gemini");
        assert!(!config.cli_name().is_empty());

        // Check default directory exists
        assert!(config.default_directory().is_some());
        assert_eq!(config.default_directory(), Some("~/.gemini/tmp"));

        // Check other properties
        assert_eq!(config.name, "Gemini CLI");
        assert_eq!(config.parser_type, ParserType::GeminiJson);
        assert!(config.supports_tokens);
        assert!(!config.file_patterns.is_empty());
    }

    #[test]
    fn test_gemini_cli_get_import_directories() {
        let config = GeminiCliConfig::create();
        let dirs = config.get_import_directories();

        // Should have at least one directory (default)
        assert!(!dirs.is_empty());
        assert!(dirs[0].contains(".gemini/tmp"));
    }

    #[tokio::test]
    async fn test_gemini_import_directories_success() {
        use std::sync::Arc;
        use std::sync::Mutex;

        let call_count = Arc::new(Mutex::new(0));
        let call_count_clone = call_count.clone();

        let import_fn = move |_path: String, _overwrite: bool| {
            let count = call_count_clone.clone();
            Box::pin(async move {
                *count.lock().unwrap() += 1;
                Ok(())
            })
                as std::pin::Pin<Box<dyn std::future::Future<Output = Result<()>> + Send>>
        };

        std::env::set_var(env_vars::GEMINI_DIRS, "/tmp/nonexistent_gemini");

        let result = GeminiCliConfig::import_directories(false, import_fn).await;
        assert!(result.is_ok());
        assert_eq!(*call_count.lock().unwrap(), 0);

        std::env::remove_var(env_vars::GEMINI_DIRS);
    }
}
