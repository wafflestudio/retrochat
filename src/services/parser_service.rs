use anyhow::{Context, Result};
use std::path::Path;

use crate::models::{ChatSession, Message};
use crate::parsers::ParserRegistry;

/// Service for parsing chat files into sessions and messages
pub struct ParserService;

impl ParserService {
    pub fn new() -> Self {
        Self
    }

    /// Parse a file and return sessions with messages
    ///
    /// This function detects the provider from the file path and uses the appropriate parser
    pub async fn parse_file(
        &self,
        file_path: impl AsRef<Path>,
    ) -> Result<Vec<(ChatSession, Vec<Message>)>> {
        let path = file_path.as_ref();

        // Detect provider and create appropriate parser
        let parser = ParserRegistry::create_parser(path)
            .with_context(|| format!("Failed to create parser for: {}", path.display()))?;

        // Parse the file
        let sessions = parser
            .parse()
            .await
            .with_context(|| format!("Failed to parse file: {}", path.display()))?;

        // Log the parsed sessions
        for (session, messages) in &sessions {
            println!(
                "📦 Parsed session: {} (provider: {}, messages: {})",
                session.id,
                session.provider,
                messages.len()
            );

            // Log session details
            if let Some(project) = &session.project_name {
                println!("   Project: {}", project);
            }
            println!("   Start time: {}", session.start_time);
            if let Some(end_time) = session.end_time {
                println!("   End time: {}", end_time);
            }
            println!("   Message count: {}", session.message_count);
            if let Some(token_count) = session.token_count {
                println!("   Token count: {}", token_count);
            }

            // Log last few messages
            let preview_count = 3.min(messages.len());
            if preview_count > 0 {
                println!("   Last {} messages:", preview_count);
                let start_index = messages.len().saturating_sub(preview_count);
                for msg in messages.iter().skip(start_index) {
                    let content_preview = if msg.content.len() > 50 {
                        format!("{}...", &msg.content[..50])
                    } else {
                        msg.content.clone()
                    };
                    println!(
                        "     - [{}] {}: {}",
                        msg.sequence_number, msg.role, content_preview
                    );
                }
            }
        }

        Ok(sessions)
    }

    /// Parse a file from a string path
    pub async fn parse_file_from_path(
        &self,
        file_path: &str,
    ) -> Result<Vec<(ChatSession, Vec<Message>)>> {
        self.parse_file(Path::new(file_path)).await
    }
}

impl Default for ParserService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[tokio::test]
    async fn test_parse_claude_code_file() {
        let mut temp_file = NamedTempFile::with_suffix(".jsonl").unwrap();
        let sample_data = r#"{"uuid":"550e8400-e29b-41d4-a716-446655440000","name":"Test Session","created_at":"2024-01-01T10:00:00Z","updated_at":"2024-01-01T11:00:00Z","chat_messages":[{"uuid":"550e8400-e29b-41d4-a716-446655440001","content":"Hello","created_at":"2024-01-01T10:00:00Z","updated_at":"2024-01-01T10:00:00Z","role":"human"},{"uuid":"550e8400-e29b-41d4-a716-446655440002","content":"Hi there!","created_at":"2024-01-01T10:01:00Z","updated_at":"2024-01-01T10:01:00Z","role":"assistant"}]}"#;
        temp_file.write_all(sample_data.as_bytes()).unwrap();

        let service = ParserService::new();
        let result = service.parse_file(temp_file.path()).await;

        assert!(result.is_ok());
        let sessions = result.unwrap();
        assert_eq!(sessions.len(), 1);

        let (session, messages) = &sessions[0];
        assert_eq!(session.message_count, 2);
        assert_eq!(messages.len(), 2);
    }
}
