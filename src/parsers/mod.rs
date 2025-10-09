pub mod claude_code;
pub mod cursor;
pub mod gemini;
pub mod project_inference;

use anyhow::{anyhow, Result};
use std::env;
use std::path::Path;

use crate::models::chat_session::LlmProvider;
use crate::models::{ChatSession, Message};

pub use claude_code::ClaudeCodeParser;
pub use cursor::CursorParser;
pub use gemini::GeminiParser;

pub enum ChatParser {
    ClaudeCode(ClaudeCodeParser),
    Cursor(CursorParser),
    Gemini(GeminiParser),
}

impl ChatParser {
    pub async fn parse(&self) -> Result<Vec<(ChatSession, Vec<Message>)>> {
        match self {
            ChatParser::ClaudeCode(parser) => {
                let (session, messages) = parser.parse().await?;
                Ok(vec![(session, messages)])
            }
            ChatParser::Cursor(parser) => {
                let (session, messages) = parser.parse().await?;
                Ok(vec![(session, messages)])
            }
            ChatParser::Gemini(parser) => parser.parse().await,
        }
    }

    pub async fn parse_streaming<F>(&self, callback: F) -> Result<()>
    where
        F: FnMut(ChatSession, Message) -> Result<()>,
    {
        match self {
            ChatParser::ClaudeCode(parser) => parser.parse_streaming(callback).await,
            ChatParser::Cursor(parser) => parser.parse_streaming(callback).await,
            ChatParser::Gemini(parser) => parser.parse_streaming(callback).await,
        }
    }

    pub fn get_provider(&self) -> LlmProvider {
        match self {
            ChatParser::ClaudeCode(_) => LlmProvider::ClaudeCode,
            ChatParser::Cursor(_) => LlmProvider::Cursor,
            ChatParser::Gemini(_) => LlmProvider::Gemini,
        }
    }
}

pub struct ParserRegistry;

impl ParserRegistry {
    pub fn detect_provider(file_path: impl AsRef<Path>) -> Option<LlmProvider> {
        let path = file_path.as_ref();

        // Check if providers are enabled via environment variables
        let claude_enabled = env::var("RETROCHAT_ENABLE_CLAUDE")
            .unwrap_or_else(|_| "true".to_string())
            .parse::<bool>()
            .unwrap_or(true);

        let gemini_enabled = env::var("RETROCHAT_ENABLE_GEMINI")
            .unwrap_or_else(|_| "true".to_string())
            .parse::<bool>()
            .unwrap_or(true);

        let cursor_enabled = env::var("RETROCHAT_ENABLE_CURSOR")
            .unwrap_or_else(|_| "true".to_string())
            .parse::<bool>()
            .unwrap_or(true);

        let codex_enabled = env::var("RETROCHAT_ENABLE_CODEX")
            .unwrap_or_else(|_| "false".to_string())
            .parse::<bool>()
            .unwrap_or(false);

        // First check by file extension and content
        if claude_enabled && ClaudeCodeParser::is_valid_file(path) {
            return Some(LlmProvider::ClaudeCode);
        }

        if cursor_enabled && CursorParser::is_valid_file(path) {
            return Some(LlmProvider::Cursor);
        }

        if gemini_enabled && GeminiParser::is_valid_file(path) {
            return Some(LlmProvider::Gemini);
        }

        // Fallback to file name patterns
        let file_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_lowercase();

        if claude_enabled && (file_name.contains("claude") || file_name.contains("anthropic")) {
            return Some(LlmProvider::ClaudeCode);
        }

        if cursor_enabled && file_name.contains("cursor") {
            return Some(LlmProvider::Cursor);
        }

        if gemini_enabled
            && (file_name.contains("gemini")
                || file_name.contains("bard")
                || file_name.contains("google"))
        {
            return Some(LlmProvider::Gemini);
        }

        if codex_enabled
            && (file_name.contains("codex")
                || file_name.contains("github")
                || file_name.contains("copilot"))
        {
            return Some(LlmProvider::Other("codex".to_string()));
        }

        // Check by file extension as last resort
        if let Some(extension) = path.extension().and_then(|e| e.to_str()) {
            match extension.to_lowercase().as_str() {
                "jsonl" if claude_enabled => Some(LlmProvider::ClaudeCode), // Default JSONL to Claude
                "json" if gemini_enabled => Some(LlmProvider::Gemini), // Default JSON to Gemini
                _ => None,
            }
        } else {
            None
        }
    }

    pub fn create_parser(file_path: impl AsRef<Path>) -> Result<ChatParser> {
        let provider = Self::detect_provider(&file_path).ok_or_else(|| {
            anyhow!(
                "Unable to detect file format for: {}",
                file_path.as_ref().display()
            )
        })?;

        match provider {
            LlmProvider::ClaudeCode => Ok(ChatParser::ClaudeCode(ClaudeCodeParser::new(file_path))),
            LlmProvider::Cursor => Ok(ChatParser::Cursor(CursorParser::new(file_path))),
            LlmProvider::Gemini => Ok(ChatParser::Gemini(GeminiParser::new(file_path))),
            LlmProvider::ChatGpt => Err(anyhow!("ChatGPT parser not yet implemented")),
            LlmProvider::Other(name) => Err(anyhow!("Parser for {name} not implemented")),
        }
    }

    pub fn get_supported_extensions() -> Vec<&'static str> {
        vec!["jsonl", "json", "db"]
    }

    pub fn get_supported_providers() -> Vec<LlmProvider> {
        vec![
            LlmProvider::ClaudeCode,
            LlmProvider::Cursor,
            LlmProvider::Gemini,
        ]
    }

    pub async fn parse_file(
        file_path: impl AsRef<Path>,
    ) -> Result<Vec<(ChatSession, Vec<Message>)>> {
        let parser = Self::create_parser(&file_path)?;
        parser.parse().await
    }

    pub async fn parse_file_streaming<F>(file_path: impl AsRef<Path>, callback: F) -> Result<()>
    where
        F: FnMut(ChatSession, Message) -> Result<()>,
    {
        let parser = Self::create_parser(&file_path)?;
        parser.parse_streaming(callback).await
    }

    pub fn scan_directory(
        directory_path: impl AsRef<Path>,
        recursive: bool,
        provider_filter: Option<&[LlmProvider]>,
    ) -> Result<Vec<(std::path::PathBuf, LlmProvider)>> {
        let mut files = Vec::new();
        Self::scan_directory_recursive(
            directory_path.as_ref(),
            recursive,
            provider_filter,
            &mut files,
        )?;
        Ok(files)
    }

    fn scan_directory_recursive(
        dir: &Path,
        recursive: bool,
        provider_filter: Option<&[LlmProvider]>,
        files: &mut Vec<(std::path::PathBuf, LlmProvider)>,
    ) -> Result<()> {
        if !dir.is_dir() {
            return Err(anyhow!("Path is not a directory: {}", dir.display()));
        }

        let entries = std::fs::read_dir(dir)?;

        for entry in entries {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() && recursive {
                Self::scan_directory_recursive(&path, recursive, provider_filter, files)?;
            } else if path.is_file() {
                if let Some(provider) = Self::detect_provider(&path) {
                    // Apply provider filter if specified
                    if let Some(filter) = provider_filter {
                        if !filter.contains(&provider) {
                            continue;
                        }
                    }
                    files.push((path, provider));
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_detect_provider_by_extension() {
        let temp_dir = TempDir::new().unwrap();

        let jsonl_file = temp_dir.path().join("test.jsonl");
        fs::write(&jsonl_file, r#"{"uuid":"test","chat_messages":[]}"#).unwrap();

        let json_file = temp_dir.path().join("test.json");
        fs::write(&json_file, r#"{"conversations":[]}"#).unwrap();

        // Note: These will return None because the files don't have valid content
        // But we can test the filename patterns
        let claude_file = temp_dir.path().join("claude_session.jsonl");
        fs::write(&claude_file, r#"{"uuid":"test","chat_messages":[]}"#).unwrap();

        let gemini_file = temp_dir.path().join("gemini_export.json");
        fs::write(&gemini_file, r#"{"conversations":[]}"#).unwrap();

        // Create Cursor test structure
        let cursor_chats = temp_dir.path().join("chats");
        let cursor_hash = cursor_chats.join("53460df9022de1a66445a5b78b067dd9");
        let cursor_uuid = cursor_hash.join("557abc41-6f00-41e7-bf7b-696c80d4ee94");
        fs::create_dir_all(&cursor_uuid).unwrap();
        let cursor_file = cursor_uuid.join("store.db");
        fs::write(&cursor_file, "").unwrap();

        assert_eq!(
            ParserRegistry::detect_provider(&claude_file),
            Some(LlmProvider::ClaudeCode)
        );
        assert_eq!(
            ParserRegistry::detect_provider(&gemini_file),
            Some(LlmProvider::Gemini)
        );
        assert_eq!(
            ParserRegistry::detect_provider(&cursor_file),
            Some(LlmProvider::Cursor)
        );
    }

    #[test]
    fn test_scan_directory() {
        let temp_dir = TempDir::new().unwrap();

        // Create test files
        let claude_file = temp_dir.path().join("claude.jsonl");
        fs::write(&claude_file, r#"{"uuid":"test","chat_messages":[]}"#).unwrap();

        let gemini_file = temp_dir.path().join("gemini.json");
        fs::write(&gemini_file, r#"{"conversations":[]}"#).unwrap();

        // Create Cursor test structure
        let cursor_chats = temp_dir.path().join("chats");
        let cursor_hash = cursor_chats.join("53460df9022de1a66445a5b78b067dd9");
        let cursor_uuid = cursor_hash.join("557abc41-6f00-41e7-bf7b-696c80d4ee94");
        fs::create_dir_all(&cursor_uuid).unwrap();
        let cursor_file = cursor_uuid.join("store.db");
        fs::write(&cursor_file, "").unwrap();

        let unknown_file = temp_dir.path().join("unknown.txt");
        fs::write(&unknown_file, "some text").unwrap();

        let result = ParserRegistry::scan_directory(temp_dir.path(), true, None).unwrap();

        // Should find 3 files (claude, gemini, and cursor)
        assert_eq!(result.len(), 3);

        let providers: Vec<_> = result.iter().map(|(_, p)| p.clone()).collect();
        assert!(providers.contains(&LlmProvider::ClaudeCode));
        assert!(providers.contains(&LlmProvider::Gemini));
        assert!(providers.contains(&LlmProvider::Cursor));
    }

    #[test]
    fn test_get_supported_info() {
        let extensions = ParserRegistry::get_supported_extensions();
        assert!(extensions.contains(&"jsonl"));
        assert!(extensions.contains(&"json"));
        assert!(extensions.contains(&"db"));

        let providers = ParserRegistry::get_supported_providers();
        assert!(providers.contains(&LlmProvider::ClaudeCode));
        assert!(providers.contains(&LlmProvider::Cursor));
        assert!(providers.contains(&LlmProvider::Gemini));
    }
}
