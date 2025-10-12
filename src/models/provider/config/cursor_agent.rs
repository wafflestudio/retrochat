use super::base::ProviderConfig;
use crate::models::provider::ParserType;
use anyhow::Result;
use std::path::Path;

pub struct CursorAgentConfig;

impl CursorAgentConfig {
    pub fn create() -> ProviderConfig {
        ProviderConfig::new("Cursor Agent".to_string(), ParserType::CursorDb)
            .with_cli_name("cursor-agent".to_string())
            .with_description("Cursor Agent (store.db files)".to_string())
            .with_env_var_name("RETROCHAT_CURSOR_DIRS".to_string())
            .with_default_directory("~/.cursor/chats".to_string())
            .with_file_patterns(vec!["store.db".to_string(), "*cursor*.db".to_string()])
            .with_default_location("darwin".to_string(), vec!["~/.cursor/chats".to_string()])
            .with_default_location("linux".to_string(), vec!["~/.cursor/chats".to_string()])
            .with_default_location(
                "windows".to_string(),
                vec!["%APPDATA%/Cursor/chats".to_string()],
            )
            .with_token_support()
            .with_tool_support()
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
            println!("  No Cursor directories found or imported");
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
            println!("  No Cursor directories found or imported");
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_cursor_agent_config() {
        let config = CursorAgentConfig::create();

        // Check CLI name exists and is correct
        assert_eq!(config.cli_name(), "cursor-agent");
        assert!(!config.cli_name().is_empty());

        // Check default directory exists
        assert!(config.default_directory().is_some());
        assert_eq!(config.default_directory(), Some("~/.cursor/chats"));

        // Check other properties
        assert_eq!(config.name, "Cursor Agent");
        assert_eq!(config.parser_type, ParserType::CursorDb);
        assert!(config.supports_tokens);
        assert!(config.supports_tools);
        assert!(!config.file_patterns.is_empty());
    }

    #[test]
    fn test_cursor_agent_get_import_directories() {
        let config = CursorAgentConfig::create();
        let dirs = config.get_import_directories();

        // Should have at least one directory (default)
        assert!(!dirs.is_empty());
        assert!(dirs[0].contains(".cursor/chats"));
    }

    #[tokio::test]
    async fn test_cursor_import_directories_success() {
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

        std::env::set_var("RETROCHAT_CURSOR_DIRS", "/tmp/nonexistent_cursor");

        let result = CursorAgentConfig::import_directories(false, import_fn).await;
        assert!(result.is_ok());
        assert_eq!(*call_count.lock().unwrap(), 0);

        std::env::remove_var("RETROCHAT_CURSOR_DIRS");
    }
}
