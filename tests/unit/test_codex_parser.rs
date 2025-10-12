use anyhow::Result;
use retrochat::models::Provider;
use retrochat::parsers::CodexParser;
use std::fs;
use std::io::Write;
use tempfile::{NamedTempFile, TempDir};

#[tokio::test]
async fn test_codex_parser_is_valid_file() {
    let mut temp_file = NamedTempFile::with_suffix(".jsonl").unwrap();
    let sample_data = r#"{"id":"test","timestamp":"2024-01-01T10:00:00Z","git":{}}"#;
    temp_file.write_all(sample_data.as_bytes()).unwrap();

    assert!(CodexParser::is_valid_file(temp_file.path()));
}

#[tokio::test]
async fn test_codex_parser_is_invalid_file() {
    // Test invalid file extension
    let mut temp_file = NamedTempFile::with_suffix(".txt").unwrap();
    temp_file.write_all(b"not json").unwrap();
    assert!(!CodexParser::is_valid_file(temp_file.path()));

    // Test invalid JSON
    let mut temp_file2 = NamedTempFile::with_suffix(".jsonl").unwrap();
    temp_file2.write_all(b"not json").unwrap();
    assert!(!CodexParser::is_valid_file(temp_file2.path()));

    // Test missing required fields
    let mut temp_file3 = NamedTempFile::with_suffix(".jsonl").unwrap();
    let invalid_data = r#"{"id":"test"}"#;
    temp_file3.write_all(invalid_data.as_bytes()).unwrap();
    assert!(!CodexParser::is_valid_file(temp_file3.path()));
}

#[tokio::test]
async fn test_codex_parser_parse() -> Result<()> {
    let mut temp_file = NamedTempFile::with_suffix(".jsonl").unwrap();
    let sample_data = r#"{"id":"550e8400-e29b-41d4-a716-446655440000","timestamp":"2024-01-01T10:00:00Z","instructions":null,"git":{"commit_hash":"abc123","branch":"main","repository_url":"git@github.com:user/test-project.git"}}
{"record_type":"state"}
{"type":"message","role":"user","content":[{"type":"input_text","text":"Hello"}]}
{"type":"message","role":"assistant","content":[{"type":"text","text":"Hi there!"}]}"#;

    temp_file.write_all(sample_data.as_bytes()).unwrap();

    let parser = CodexParser::new(temp_file.path());
    let result = parser.parse().await;

    assert!(result.is_ok());
    let (session, messages) = result.unwrap();

    assert_eq!(session.provider, Provider::Codex);
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
async fn test_codex_parser_parse_streaming() -> Result<()> {
    let mut temp_file = NamedTempFile::with_suffix(".jsonl").unwrap();
    let sample_data = r#"{"id":"550e8400-e29b-41d4-a716-446655440000","timestamp":"2024-01-01T10:00:00Z","git":{}}
{"type":"message","role":"user","content":[{"type":"input_text","text":"Hello"}]}"#;

    temp_file.write_all(sample_data.as_bytes()).unwrap();

    let parser = CodexParser::new(temp_file.path());

    let mut session_count = 0;
    let mut message_count = 0;

    parser
        .parse_streaming(|session, _message| {
            session_count += 1;
            message_count += 1;
            assert_eq!(session.provider, Provider::Codex);
            Ok(())
        })
        .await?;

    assert_eq!(session_count, 1);
    assert_eq!(message_count, 1);

    Ok(())
}

#[tokio::test]
async fn test_codex_parser_project_from_git() -> Result<()> {
    let mut temp_file = NamedTempFile::with_suffix(".jsonl").unwrap();
    let sample_data = r#"{"id":"550e8400-e29b-41d4-a716-446655440000","timestamp":"2024-01-01T10:00:00Z","git":{"commit_hash":"abc123","branch":"main","repository_url":"git@github.com:user/test-project.git"}}
{"type":"message","role":"user","content":[{"type":"input_text","text":"Hello"}]}"#;

    temp_file.write_all(sample_data.as_bytes()).unwrap();

    let parser = CodexParser::new(temp_file.path());
    let result = parser.parse().await;

    assert!(result.is_ok());
    let (session, _messages) = result.unwrap();

    // Should have extracted project name from git URL
    assert_eq!(session.project_name, Some("test-project".to_string()));

    Ok(())
}

#[tokio::test]
async fn test_codex_parser_project_inference() {
    let temp_dir = TempDir::new().unwrap();
    let base_path = temp_dir.path();

    // Create a project directory
    let project_dir = base_path.join("testproject");
    fs::create_dir_all(&project_dir).unwrap();

    let test_file = project_dir.join("test.jsonl");

    let sample_data = r#"{"id":"550e8400-e29b-41d4-a716-446655440000","timestamp":"2024-01-01T10:00:00Z","git":{}}
{"type":"message","role":"user","content":[{"type":"input_text","text":"Hello"}]}"#;
    fs::write(&test_file, sample_data).unwrap();

    let parser = CodexParser::new(&test_file);
    let result = parser.parse().await;

    assert!(result.is_ok());
    let (session, _messages) = result.unwrap();

    // Should have inferred the project name from the path
    assert_eq!(session.project_name, Some("testproject".to_string()));
}

#[tokio::test]
async fn test_codex_parser_empty_content() -> Result<()> {
    let mut temp_file = NamedTempFile::with_suffix(".jsonl").unwrap();
    let sample_data = r#"{"id":"550e8400-e29b-41d4-a716-446655440000","timestamp":"2024-01-01T10:00:00Z","git":{}}
{"type":"message","role":"user","content":[]}"#;

    temp_file.write_all(sample_data.as_bytes()).unwrap();

    let parser = CodexParser::new(temp_file.path());
    let result = parser.parse().await;

    assert!(result.is_ok());
    let (session, messages) = result.unwrap();

    assert_eq!(session.message_count, 1);
    // Empty content should be replaced with "[No content]"
    assert_eq!(messages[0].content, "[No content]");

    Ok(())
}

#[tokio::test]
async fn test_codex_parser_skip_state_records() -> Result<()> {
    let mut temp_file = NamedTempFile::with_suffix(".jsonl").unwrap();
    let sample_data = r#"{"id":"550e8400-e29b-41d4-a716-446655440000","timestamp":"2024-01-01T10:00:00Z","git":{}}
{"record_type":"state"}
{"record_type":"state"}
{"type":"message","role":"user","content":[{"type":"input_text","text":"Hello"}]}"#;

    temp_file.write_all(sample_data.as_bytes()).unwrap();

    let parser = CodexParser::new(temp_file.path());
    let result = parser.parse().await;

    assert!(result.is_ok());
    let (session, messages) = result.unwrap();

    // State records should be skipped, only 1 message should be parsed
    assert_eq!(session.message_count, 1);
    assert_eq!(messages.len(), 1);

    Ok(())
}

#[tokio::test]
async fn test_codex_parser_invalid_uuid() {
    let mut temp_file = NamedTempFile::with_suffix(".jsonl").unwrap();
    let sample_data = r#"{"id":"invalid-uuid","timestamp":"2024-01-01T10:00:00Z","git":{}}"#;

    temp_file.write_all(sample_data.as_bytes()).unwrap();

    let parser = CodexParser::new(temp_file.path());
    let result = parser.parse().await;

    assert!(result.is_err());
}

#[tokio::test]
async fn test_codex_parser_missing_header() {
    let mut temp_file = NamedTempFile::with_suffix(".jsonl").unwrap();
    let sample_data =
        r#"{"type":"message","role":"user","content":[{"type":"input_text","text":"Hello"}]}"#;

    temp_file.write_all(sample_data.as_bytes()).unwrap();

    let parser = CodexParser::new(temp_file.path());
    let result = parser.parse().await;

    assert!(result.is_err());
}

#[tokio::test]
async fn test_codex_parser_file_consistency() {
    let mut temp_file = NamedTempFile::with_suffix(".jsonl").unwrap();
    let sample_data = r#"{"id":"550e8400-e29b-41d4-a716-446655440000","timestamp":"2024-01-01T10:00:00Z","git":{}}
{"type":"message","role":"user","content":[{"type":"input_text","text":"Hello"}]}"#;

    temp_file.write_all(sample_data.as_bytes()).unwrap();

    let parser1 = CodexParser::new(temp_file.path());
    let parser2 = CodexParser::new(temp_file.path());

    let (session1, _) = parser1.parse().await.unwrap();
    let (session2, _) = parser2.parse().await.unwrap();

    // File hash should be consistent between parsers
    assert_eq!(session1.file_hash, session2.file_hash);
    assert!(!session1.file_hash.is_empty());
}

#[tokio::test]
async fn test_codex_parser_multiple_content_items() -> Result<()> {
    let mut temp_file = NamedTempFile::with_suffix(".jsonl").unwrap();
    let sample_data = r#"{"id":"550e8400-e29b-41d4-a716-446655440000","timestamp":"2024-01-01T10:00:00Z","git":{}}
{"type":"message","role":"user","content":[{"type":"input_text","text":"Hello"},{"type":"input_text","text":"World"}]}"#;

    temp_file.write_all(sample_data.as_bytes()).unwrap();

    let parser = CodexParser::new(temp_file.path());
    let result = parser.parse().await;

    assert!(result.is_ok());
    let (session, messages) = result.unwrap();

    assert_eq!(session.message_count, 1);
    // Multiple content items should be joined with newlines
    assert!(messages[0].content.contains("Hello"));
    assert!(messages[0].content.contains("World"));

    Ok(())
}
