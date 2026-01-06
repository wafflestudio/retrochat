pub mod claude_code;
pub mod codex;
pub mod cursor_client;
pub mod gemini_cli;
pub mod project_inference;

use anyhow::{anyhow, Result};
use std::path::Path;

use crate::models::Provider;
use crate::models::{ChatSession, Message};

pub use claude_code::ClaudeCodeParser;
pub use codex::CodexParser;
pub use cursor_client::CursorClientParser;
pub use gemini_cli::GeminiCLIParser;

pub enum ChatParser {
    ClaudeCode(ClaudeCodeParser),
    Codex(CodexParser),
    CursorClient(CursorClientParser),
    GeminiCLI(GeminiCLIParser),
}

impl ChatParser {
    pub async fn parse(&self) -> Result<Vec<(ChatSession, Vec<Message>)>> {
        match self {
            ChatParser::ClaudeCode(parser) => {
                let (session, messages) = parser.parse().await?;
                Ok(vec![(session, messages)])
            }
            ChatParser::Codex(parser) => {
                let (session, messages) = parser.parse().await?;
                Ok(vec![(session, messages)])
            }
            ChatParser::CursorClient(parser) => parser.parse().await,
            ChatParser::GeminiCLI(parser) => parser.parse().await,
        }
    }

    pub async fn parse_streaming<F>(&self, callback: F) -> Result<()>
    where
        F: FnMut(ChatSession, Message) -> Result<()>,
    {
        match self {
            ChatParser::ClaudeCode(parser) => parser.parse_streaming(callback).await,
            ChatParser::Codex(parser) => parser.parse_streaming(callback).await,
            ChatParser::CursorClient(parser) => parser.parse_streaming(callback).await,
            ChatParser::GeminiCLI(parser) => parser.parse_streaming(callback).await,
        }
    }

    pub fn get_provider(&self) -> Provider {
        match self {
            ChatParser::ClaudeCode(_) => Provider::ClaudeCode,
            ChatParser::Codex(_) => Provider::Codex,
            ChatParser::CursorClient(_) => Provider::CursorClient,
            ChatParser::GeminiCLI(_) => Provider::GeminiCLI,
        }
    }
}

pub struct ParserRegistry;

impl ParserRegistry {
    pub fn detect_provider(file_path: impl AsRef<Path>) -> Option<Provider> {
        let path = file_path.as_ref();

        // First check by file extension and content
        // is_valid_file() already includes filename filtering via accepts_filename()
        if ClaudeCodeParser::is_valid_file(path) {
            return Some(Provider::ClaudeCode);
        }

        if CodexParser::is_valid_file(path) {
            return Some(Provider::Codex);
        }

        if CursorClientParser::is_valid_file(path) {
            return Some(Provider::CursorClient);
        }

        if GeminiCLIParser::is_valid_file(path) {
            return Some(Provider::GeminiCLI);
        }

        // Fallback to file name patterns (with filename filtering)
        // Note: For fallback, we need to check accepts_filename() separately
        // since is_valid_file() may fail on content checks
        let file_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_lowercase();

        if (file_name.contains("claude") || file_name.contains("anthropic"))
            && ClaudeCodeParser::accepts_filename(path)
        {
            return Some(Provider::ClaudeCode);
        }

        if (file_name.contains("gemini")
            || file_name.contains("bard")
            || file_name.contains("google"))
            && GeminiCLIParser::accepts_filename(path)
        {
            return Some(Provider::GeminiCLI);
        }

        if file_name.contains("codex")
            || file_name.contains("github")
            || file_name.contains("copilot")
        {
            // Codex doesn't enforce filename filtering in fallback
            return Some(Provider::Other("codex".to_string()));
        }

        // Check by file extension as last resort (with filename filtering)
        if let Some(extension) = path.extension().and_then(|e| e.to_str()) {
            match extension.to_lowercase().as_str() {
                "jsonl" => {
                    if ClaudeCodeParser::accepts_filename(path) {
                        return Some(Provider::ClaudeCode);
                    }
                }
                "json" => {
                    if GeminiCLIParser::accepts_filename(path) {
                        return Some(Provider::GeminiCLI);
                    }
                }
                _ => {}
            }
        }

        None
    }

    pub fn create_parser(file_path: impl AsRef<Path>) -> Result<ChatParser> {
        let provider = Self::detect_provider(&file_path).ok_or_else(|| {
            anyhow!(
                "Unable to detect file format for: {}",
                file_path.as_ref().display()
            )
        })?;

        match provider {
            Provider::ClaudeCode => Ok(ChatParser::ClaudeCode(ClaudeCodeParser::new(file_path))),
            Provider::Codex => Ok(ChatParser::Codex(CodexParser::new(file_path))),
            Provider::CursorClient => {
                Ok(ChatParser::CursorClient(CursorClientParser::new(file_path)))
            }
            Provider::GeminiCLI => Ok(ChatParser::GeminiCLI(GeminiCLIParser::new(file_path))),
            Provider::All => Err(anyhow!(
                "'All' is a CLI-only provider and cannot be used for parsing"
            )),
            Provider::Other(name) => Err(anyhow!("Parser for {name} not implemented")),
        }
    }

    pub fn get_supported_extensions() -> Vec<&'static str> {
        vec!["jsonl", "json", "db", "vscdb"]
    }

    pub fn get_supported_providers() -> Vec<Provider> {
        vec![
            Provider::ClaudeCode,
            Provider::Codex,
            Provider::CursorClient,
            Provider::GeminiCLI,
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
        provider_filter: Option<&[Provider]>,
    ) -> Result<Vec<(std::path::PathBuf, Provider)>> {
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
        provider_filter: Option<&[Provider]>,
        files: &mut Vec<(std::path::PathBuf, Provider)>,
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

        // Use UUID format filename for Claude Code (required by accepts_filename)
        let claude_file = temp_dir
            .path()
            .join("550e8400-e29b-41d4-a716-446655440000.jsonl");
        fs::write(&claude_file, r#"{"uuid":"test","chat_messages":[]}"#).unwrap();

        // Use "session-" prefix for Gemini CLI (required by accepts_filename)
        let gemini_file = temp_dir.path().join("session-test.json");
        fs::write(&gemini_file, r#"{"conversations":[]}"#).unwrap();

        assert_eq!(
            ParserRegistry::detect_provider(&claude_file),
            Some(Provider::ClaudeCode)
        );
        assert_eq!(
            ParserRegistry::detect_provider(&gemini_file),
            Some(Provider::GeminiCLI)
        );
    }

    #[test]
    fn test_scan_directory() {
        let temp_dir = TempDir::new().unwrap();

        // Create test files with proper naming conventions
        // UUID format for Claude Code
        let claude_file = temp_dir
            .path()
            .join("550e8400-e29b-41d4-a716-446655440000.jsonl");
        fs::write(&claude_file, r#"{"uuid":"test","chat_messages":[]}"#).unwrap();

        // "session-" prefix for Gemini CLI
        let gemini_file = temp_dir.path().join("session-test.json");
        fs::write(&gemini_file, r#"{"conversations":[]}"#).unwrap();

        let unknown_file = temp_dir.path().join("unknown.txt");
        fs::write(&unknown_file, "some text").unwrap();

        let result = ParserRegistry::scan_directory(temp_dir.path(), true, None).unwrap();

        // Should find 2 files (claude and gemini)
        assert_eq!(result.len(), 2);

        let providers: Vec<_> = result.iter().map(|(_, p)| p.clone()).collect();
        assert!(providers.contains(&Provider::ClaudeCode));
        assert!(providers.contains(&Provider::GeminiCLI));
    }

    #[test]
    fn test_get_supported_info() {
        let extensions = ParserRegistry::get_supported_extensions();
        assert!(extensions.contains(&"jsonl"));
        assert!(extensions.contains(&"json"));
        assert!(extensions.contains(&"db"));

        let providers = ParserRegistry::get_supported_providers();
        assert!(providers.contains(&Provider::ClaudeCode));
        assert!(providers.contains(&Provider::GeminiCLI));
    }
}
