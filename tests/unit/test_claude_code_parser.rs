use anyhow::Result;
use retrochat::models::Provider;
use retrochat::parsers::ClaudeCodeParser;
use std::fs;
use std::io::Write;
use tempfile::{NamedTempFile, TempDir};

#[tokio::test]
async fn test_claude_code_parser_is_valid_file() {
    let mut temp_file = NamedTempFile::with_suffix(".jsonl").unwrap();
    let sample_data = r#"{"uuid":"test","chat_messages":[]}"#;
    temp_file.write_all(sample_data.as_bytes()).unwrap();

    assert!(ClaudeCodeParser::is_valid_file(temp_file.path()));
}

#[tokio::test]
async fn test_claude_code_parser_is_invalid_file() {
    // Test invalid file extension
    let mut temp_file = NamedTempFile::with_suffix(".txt").unwrap();
    temp_file.write_all(b"not json").unwrap();
    assert!(!ClaudeCodeParser::is_valid_file(temp_file.path()));

    // Test invalid JSON
    let mut temp_file2 = NamedTempFile::with_suffix(".jsonl").unwrap();
    temp_file2.write_all(b"not json").unwrap();
    assert!(!ClaudeCodeParser::is_valid_file(temp_file2.path()));
}

#[tokio::test]
async fn test_claude_code_parser_parse_session_format() -> Result<()> {
    let mut temp_file = NamedTempFile::with_suffix(".jsonl").unwrap();
    let sample_data = r#"{"uuid":"550e8400-e29b-41d4-a716-446655440000","name":"Test Session","created_at":"2024-01-01T10:00:00Z","updated_at":"2024-01-01T11:00:00Z","chat_messages":[{"uuid":"550e8400-e29b-41d4-a716-446655440001","content":"Hello","created_at":"2024-01-01T10:00:00Z","updated_at":"2024-01-01T10:00:00Z","role":"human"},{"uuid":"550e8400-e29b-41d4-a716-446655440002","content":"Hi there!","created_at":"2024-01-01T10:01:00Z","updated_at":"2024-01-01T10:01:00Z","role":"assistant"}]}"#;

    temp_file.write_all(sample_data.as_bytes()).unwrap();

    let parser = ClaudeCodeParser::new(temp_file.path());
    let result = parser.parse().await;

    assert!(result.is_ok());
    let (session, messages) = result.unwrap();

    assert_eq!(session.provider, Provider::ClaudeCode);
    assert_eq!(
        session.id.to_string(),
        "550e8400-e29b-41d4-a716-446655440000"
    );
    assert_eq!(session.message_count, 2);
    assert_eq!(messages.len(), 2);
    assert_eq!(messages[0].content, "Hello");
    assert_eq!(messages[1].content, "Hi there!");

    Ok(())
}

#[tokio::test]
async fn test_claude_code_parser_parse_conversation_format() -> Result<()> {
    let mut temp_file = NamedTempFile::with_suffix(".jsonl").unwrap();
    let sample_data = r#"{"type":"conversation","sessionId":"550e8400-e29b-41d4-a716-446655440000","timestamp":"2024-01-01T10:00:00Z","message":{"role":"user","content":"Hello"}}
{"type":"conversation","sessionId":"550e8400-e29b-41d4-a716-446655440000","timestamp":"2024-01-01T10:01:00Z","message":{"role":"assistant","content":"Hi there!"}}"#;

    temp_file.write_all(sample_data.as_bytes()).unwrap();

    let parser = ClaudeCodeParser::new(temp_file.path());
    let result = parser.parse().await;

    assert!(result.is_ok());
    let (session, messages) = result.unwrap();

    assert_eq!(session.provider, Provider::ClaudeCode);
    assert_eq!(
        session.id.to_string(),
        "550e8400-e29b-41d4-a716-446655440000"
    );
    assert_eq!(session.message_count, 2);
    assert_eq!(messages.len(), 2);

    Ok(())
}

#[tokio::test]
async fn test_claude_code_parser_parse_streaming() -> Result<()> {
    let mut temp_file = NamedTempFile::with_suffix(".jsonl").unwrap();
    let sample_data = r#"{"uuid":"550e8400-e29b-41d4-a716-446655440000","name":"Test","created_at":"2024-01-01T10:00:00Z","updated_at":"2024-01-01T10:00:00Z","chat_messages":[{"uuid":"550e8400-e29b-41d4-a716-446655440001","content":"Hello","created_at":"2024-01-01T10:00:00Z","updated_at":"2024-01-01T10:00:00Z","role":"user"}]}"#;

    temp_file.write_all(sample_data.as_bytes()).unwrap();

    let parser = ClaudeCodeParser::new(temp_file.path());

    let mut session_count = 0;
    let mut message_count = 0;

    parser
        .parse_streaming(|session, _message| {
            session_count += 1;
            message_count += 1;
            assert_eq!(session.provider, Provider::ClaudeCode);
            Ok(())
        })
        .await?;

    assert_eq!(session_count, 1);
    assert_eq!(message_count, 1);

    Ok(())
}

#[tokio::test]
async fn test_claude_code_parser_complex_content() -> Result<()> {
    let mut temp_file = NamedTempFile::with_suffix(".jsonl").unwrap();
    let sample_data = r#"{"uuid":"550e8400-e29b-41d4-a716-446655440000","created_at":"2024-01-01T10:00:00Z","updated_at":"2024-01-01T10:00:00Z","chat_messages":[{"uuid":"550e8400-e29b-41d4-a716-446655440001","content":[{"text":"Hello"}],"created_at":"2024-01-01T10:00:00Z","updated_at":"2024-01-01T10:00:00Z","role":"user"}]}"#;

    temp_file.write_all(sample_data.as_bytes()).unwrap();

    let parser = ClaudeCodeParser::new(temp_file.path());
    let result = parser.parse().await;

    assert!(result.is_ok());
    let (session, messages) = result.unwrap();

    assert_eq!(session.message_count, 1);
    assert_eq!(messages.len(), 1);
    assert!(messages[0].content.contains("Hello"));

    Ok(())
}

#[tokio::test]
async fn test_claude_code_parser_empty_file() {
    let temp_file = NamedTempFile::with_suffix(".jsonl").unwrap();

    let parser = ClaudeCodeParser::new(temp_file.path());
    let result = parser.parse().await;

    assert!(result.is_err());
}

#[tokio::test]
async fn test_claude_code_parser_invalid_uuid() {
    let mut temp_file = NamedTempFile::with_suffix(".jsonl").unwrap();
    let sample_data = r#"{"uuid":"invalid-uuid","created_at":"2024-01-01T10:00:00Z","updated_at":"2024-01-01T10:00:00Z","chat_messages":[]}"#;

    temp_file.write_all(sample_data.as_bytes()).unwrap();

    let parser = ClaudeCodeParser::new(temp_file.path());
    let result = parser.parse().await;

    assert!(result.is_err());
}

#[tokio::test]
async fn test_claude_code_parser_project_inference() {
    let temp_dir = TempDir::new().unwrap();
    let base_path = temp_dir.path();

    // Create the actual project directory structure
    let project_path = base_path
        .join("Users")
        .join("testuser")
        .join("Project")
        .join("testproject");
    fs::create_dir_all(&project_path).unwrap();

    // Create Claude's encoded directory
    let claude_dir = base_path.join("-Users-testuser-Project-testproject");
    fs::create_dir_all(&claude_dir).unwrap();

    let test_file = claude_dir.join("test.jsonl");

    // Create a sample conversation without explicit project name
    let sample_data = r#"{"type":"conversation","sessionId":"550e8400-e29b-41d4-a716-446655440000","timestamp":"2024-01-01T10:00:00Z","message":{"role":"user","content":"Hello"}}"#;
    fs::write(&test_file, sample_data).unwrap();

    let parser = ClaudeCodeParser::new(&test_file);
    let result = parser.parse().await;

    assert!(result.is_ok());
    let (session, _messages) = result.unwrap();

    // Should have inferred the project name from the path
    assert_eq!(session.project_name, Some("testproject".to_string()));
}

#[tokio::test]
async fn test_claude_code_parser_file_consistency() {
    let mut temp_file = NamedTempFile::with_suffix(".jsonl").unwrap();
    let sample_data = r#"{"uuid":"550e8400-e29b-41d4-a716-446655440000","created_at":"2024-01-01T10:00:00Z","updated_at":"2024-01-01T10:00:00Z","chat_messages":[{"uuid":"550e8400-e29b-41d4-a716-446655440001","content":"Hello","created_at":"2024-01-01T10:00:00Z","updated_at":"2024-01-01T10:00:00Z","role":"user"}]}"#;

    temp_file.write_all(sample_data.as_bytes()).unwrap();

    let parser1 = ClaudeCodeParser::new(temp_file.path());
    let parser2 = ClaudeCodeParser::new(temp_file.path());

    let (session1, _) = parser1.parse().await.unwrap();
    let (session2, _) = parser2.parse().await.unwrap();

    // File hash should be consistent between parsers
    assert_eq!(session1.file_hash, session2.file_hash);
    assert!(!session1.file_hash.is_empty());
}
