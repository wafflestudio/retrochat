use super::base::ProviderConfig;
use crate::models::provider::ParserType;
use anyhow::Result;
use std::path::Path;

pub struct ClaudeCodeConfig;

impl ClaudeCodeConfig {
    pub fn create() -> ProviderConfig {
        ProviderConfig::new("Claude Code".to_string(), ParserType::ClaudeCodeJsonl)
            .with_cli_name("claude".to_string())
            .with_description("Claude Code (.jsonl files)".to_string())
            .with_env_var_name("RETROCHAT_CLAUDE_DIRS".to_string())
            .with_default_directory("~/.claude/projects".to_string())
            .with_file_patterns(vec![
                "*.jsonl".to_string(),
                "*claude-code*.json*".to_string(),
            ])
            .with_default_location("darwin".to_string(), vec!["~/.claude/projects".to_string()])
            .with_default_location("linux".to_string(), vec!["~/.claude/projects".to_string()])
            .with_default_location(
                "windows".to_string(),
                vec!["%APPDATA%/Claude Code/".to_string()],
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
            println!("  No Claude directories found or imported");
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
            println!("  No Claude directories found or imported");
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_claude_code_config() {
        let config = ClaudeCodeConfig::create();

        // Check CLI name exists and is correct
        assert_eq!(config.cli_name(), "claude");
        assert!(!config.cli_name().is_empty());

        // Check default directory exists
        assert!(config.default_directory().is_some());
        assert_eq!(config.default_directory(), Some("~/.claude/projects"));

        // Check other properties
        assert_eq!(config.name, "Claude Code");
        assert_eq!(config.parser_type, ParserType::ClaudeCodeJsonl);
        assert!(config.supports_tokens);
        assert!(config.supports_tools);
        assert!(!config.file_patterns.is_empty());
    }

    #[test]
    fn test_claude_code_get_import_directories() {
        let config = ClaudeCodeConfig::create();
        let dirs = config.get_import_directories();

        // Should have at least one directory (default)
        assert!(!dirs.is_empty());
        assert!(dirs[0].contains(".claude/projects"));
    }

    #[test]
    fn test_claude_code_import_directories_with_env() {
        std::env::set_var("RETROCHAT_CLAUDE_DIRS", "/tmp/test1:/tmp/test2");
        let config = ClaudeCodeConfig::create();
        let dirs = config.get_import_directories();

        assert_eq!(dirs.len(), 2);
        assert_eq!(dirs[0], "/tmp/test1");
        assert_eq!(dirs[1], "/tmp/test2");

        std::env::remove_var("RETROCHAT_CLAUDE_DIRS");
    }

    #[tokio::test]
    async fn test_import_directories_success() {
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

        // Set a test directory that doesn't exist
        std::env::set_var("RETROCHAT_CLAUDE_DIRS", "/tmp/nonexistent_test_dir");

        let result = ClaudeCodeConfig::import_directories(false, import_fn).await;
        assert!(result.is_ok());

        // Should not have been called since directory doesn't exist
        assert_eq!(*call_count.lock().unwrap(), 0);

        std::env::remove_var("RETROCHAT_CLAUDE_DIRS");
    }
}
