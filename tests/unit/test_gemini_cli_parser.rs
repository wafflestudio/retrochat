use anyhow::Result;
use retrochat::models::Provider;
use retrochat::parsers::GeminiCLIParser;
use std::fs;
use std::io::Write;
use tempfile::{NamedTempFile, TempDir};

#[tokio::test]
async fn test_gemini_parser_is_valid_file() {
    let mut temp_file = NamedTempFile::with_suffix(".json").unwrap();
    let sample_data = r#"{"conversations":[]}"#;
    temp_file.write_all(sample_data.as_bytes()).unwrap();

    assert!(GeminiCLIParser::is_valid_file(temp_file.path()));
}

#[tokio::test]
async fn test_gemini_parser_is_invalid_file() {
    // Test invalid file extension
    let mut temp_file = NamedTempFile::with_suffix(".txt").unwrap();
    temp_file.write_all(b"not json").unwrap();
    assert!(!GeminiCLIParser::is_valid_file(temp_file.path()));

    // Test invalid JSON
    let mut temp_file2 = NamedTempFile::with_suffix(".json").unwrap();
    temp_file2.write_all(b"not json").unwrap();
    assert!(!GeminiCLIParser::is_valid_file(temp_file2.path()));
}

#[tokio::test]
async fn test_gemini_parser_parse_export_format() -> Result<()> {
    let mut temp_file = NamedTempFile::with_suffix(".json").unwrap();
    let sample_data = r#"{"conversations":[{"conversation_id":"test-123","create_time":"2024-01-01T10:00:00Z","update_time":"2024-01-01T11:00:00Z","title":"Test Chat","conversation":[{"parts":[{"text":"Hello"}],"role":"user","timestamp":"2024-01-01T10:00:00Z"},{"parts":[{"text":"Hi there!"}],"role":"model","timestamp":"2024-01-01T10:01:00Z"}]}]}"#;

    temp_file.write_all(sample_data.as_bytes()).unwrap();

    let parser = GeminiCLIParser::new(temp_file.path());
    let result = parser.parse().await;

    assert!(result.is_ok());
    let sessions = result.unwrap();

    assert_eq!(sessions.len(), 1);

    let (session, messages) = &sessions[0];
    assert_eq!(session.provider, Provider::GeminiCLI);
    assert_eq!(session.message_count, 2);
    assert_eq!(messages.len(), 2);
    assert_eq!(messages[0].content, "Hello");
    assert_eq!(messages[1].content, "Hi there!");

    Ok(())
}

#[tokio::test]
async fn test_gemini_parser_parse_session_format() -> Result<()> {
    let mut temp_file = NamedTempFile::with_suffix(".json").unwrap();
    let sample_data = r#"{"sessionId":"550e8400-e29b-41d4-a716-446655440000","projectHash":"abc123","startTime":"2024-01-01T10:00:00Z","lastUpdated":"2024-01-01T11:00:00Z","messages":[{"id":"msg-1","timestamp":"2024-01-01T10:00:00Z","type":"user","content":"Hello"},{"id":"msg-2","timestamp":"2024-01-01T10:01:00Z","type":"gemini","content":"Hi there!"}]}"#;

    temp_file.write_all(sample_data.as_bytes()).unwrap();

    let parser = GeminiCLIParser::new(temp_file.path());
    let result = parser.parse().await;

    assert!(result.is_ok());
    let sessions = result.unwrap();

    assert_eq!(sessions.len(), 1);

    let (session, messages) = &sessions[0];
    assert_eq!(session.provider, Provider::GeminiCLI);
    assert_eq!(
        session.id.to_string(),
        "550e8400-e29b-41d4-a716-446655440000"
    );
    assert_eq!(session.message_count, 2);
    assert_eq!(messages.len(), 2);

    Ok(())
}

#[tokio::test]
async fn test_gemini_parser_parse_streaming() -> Result<()> {
    let mut temp_file = NamedTempFile::with_suffix(".json").unwrap();
    let sample_data =
        r#"{"conversations":[{"conversation":[{"parts":[{"text":"Hello"}],"role":"user"}]}]}"#;

    temp_file.write_all(sample_data.as_bytes()).unwrap();

    let parser = GeminiCLIParser::new(temp_file.path());

    let mut session_count = 0;
    let mut message_count = 0;

    parser
        .parse_streaming(|session, _message| {
            session_count += 1;
            message_count += 1;
            assert_eq!(session.provider, Provider::GeminiCLI);
            Ok(())
        })
        .await?;

    assert_eq!(session_count, 1);
    assert_eq!(message_count, 1);

    Ok(())
}

#[tokio::test]
async fn test_gemini_parser_multiple_conversations() -> Result<()> {
    let mut temp_file = NamedTempFile::with_suffix(".json").unwrap();
    let sample_data = r#"{"conversations":[{"conversation":[{"parts":[{"text":"Hello"}],"role":"user"}]},{"conversation":[{"parts":[{"text":"World"}],"role":"user"}]}]}"#;

    temp_file.write_all(sample_data.as_bytes()).unwrap();

    let parser = GeminiCLIParser::new(temp_file.path());
    let result = parser.parse().await;

    assert!(result.is_ok());
    let sessions = result.unwrap();

    assert_eq!(sessions.len(), 2);

    Ok(())
}

#[tokio::test]
async fn test_gemini_parser_get_conversation_count() -> Result<()> {
    let mut temp_file = NamedTempFile::with_suffix(".json").unwrap();
    let sample_data =
        r#"{"conversations":[{"conversation":[]},{"conversation":[]},{"conversation":[]}]}"#;
    temp_file.write_all(sample_data.as_bytes()).unwrap();

    let parser = GeminiCLIParser::new(temp_file.path());
    let count = parser.get_conversation_count()?;

    assert_eq!(count, 3);

    Ok(())
}

#[tokio::test]
async fn test_gemini_parser_empty_file() {
    let mut temp_file = NamedTempFile::with_suffix(".json").unwrap();
    let sample_data = r#"{"conversations":[]}"#;
    temp_file.write_all(sample_data.as_bytes()).unwrap();

    let parser = GeminiCLIParser::new(temp_file.path());
    let result = parser.parse().await;

    assert!(result.is_err());
}

#[tokio::test]
async fn test_gemini_parser_with_tokens() -> Result<()> {
    let mut temp_file = NamedTempFile::with_suffix(".json").unwrap();
    let sample_data = r#"{"sessionId":"550e8400-e29b-41d4-a716-446655440000","startTime":"2024-01-01T10:00:00Z","lastUpdated":"2024-01-01T10:00:00Z","messages":[{"id":"msg-1","timestamp":"2024-01-01T10:00:00Z","type":"user","content":"Hello","tokens":{"input":10,"output":0,"cached":0,"thoughts":0,"tool":0,"total":10}}]}"#;

    temp_file.write_all(sample_data.as_bytes()).unwrap();

    let parser = GeminiCLIParser::new(temp_file.path());
    let result = parser.parse().await;

    assert!(result.is_ok());
    let sessions = result.unwrap();
    let (session, messages) = &sessions[0];

    assert_eq!(session.message_count, 1);
    assert_eq!(messages[0].token_count, Some(10));

    Ok(())
}

#[tokio::test]
async fn test_gemini_parser_project_inference() {
    let temp_dir = TempDir::new().unwrap();
    let base_path = temp_dir.path();

    // Create a project directory
    let project_dir = base_path.join("testproject");
    fs::create_dir_all(&project_dir).unwrap();

    let test_file = project_dir.join("test.json");

    let sample_data = r#"{"sessionId":"550e8400-e29b-41d4-a716-446655440000","startTime":"2024-01-01T10:00:00Z","lastUpdated":"2024-01-01T10:00:00Z","messages":[{"id":"msg-1","timestamp":"2024-01-01T10:00:00Z","type":"user","content":"Hello"}]}"#;
    fs::write(&test_file, sample_data).unwrap();

    let parser = GeminiCLIParser::new(&test_file);
    let result = parser.parse().await;

    assert!(result.is_ok());
    let sessions = result.unwrap();
    let (session, _messages) = &sessions[0];

    // Should have inferred the project name from the path
    assert_eq!(session.project_name, Some("testproject".to_string()));
}

#[tokio::test]
async fn test_gemini_parser_file_consistency() {
    let mut temp_file = NamedTempFile::with_suffix(".json").unwrap();
    let sample_data = r#"{"sessionId":"550e8400-e29b-41d4-a716-446655440000","startTime":"2024-01-01T10:00:00Z","lastUpdated":"2024-01-01T10:00:00Z","messages":[{"id":"msg-1","timestamp":"2024-01-01T10:00:00Z","type":"user","content":"Hello"}]}"#;

    temp_file.write_all(sample_data.as_bytes()).unwrap();

    let parser1 = GeminiCLIParser::new(temp_file.path());
    let parser2 = GeminiCLIParser::new(temp_file.path());

    let sessions1 = parser1.parse().await.unwrap();
    let sessions2 = parser2.parse().await.unwrap();

    let (session1, _) = &sessions1[0];
    let (session2, _) = &sessions2[0];

    // File hash should be consistent between parsers
    assert_eq!(session1.file_hash, session2.file_hash);
    assert!(!session1.file_hash.is_empty());
}

#[tokio::test]
async fn test_gemini_parser_invalid_json() {
    let mut temp_file = NamedTempFile::with_suffix(".json").unwrap();
    temp_file.write_all(b"invalid json").unwrap();

    let parser = GeminiCLIParser::new(temp_file.path());
    let result = parser.parse().await;

    assert!(result.is_err());
}
