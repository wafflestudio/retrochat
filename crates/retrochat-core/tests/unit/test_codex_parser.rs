use anyhow::Result;
use retrochat_core::models::Provider;
use retrochat_core::parsers::CodexParser;
use std::fs;
use std::io::Write;
use tempfile::{NamedTempFile, TempDir};

#[tokio::test]
async fn test_codex_parser_is_valid_file() {
    let mut temp_file = NamedTempFile::with_suffix(".jsonl").unwrap();
    let sample_data = r#"{"timestamp":"2025-10-12T14:10:16.717Z","type":"session_meta","payload":{"id":"test","timestamp":"2024-01-01T10:00:00Z"}}"#;
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

    // Test missing session_meta
    let mut temp_file3 = NamedTempFile::with_suffix(".jsonl").unwrap();
    let invalid_data = r#"{"type":"other","payload":{}}"#;
    temp_file3.write_all(invalid_data.as_bytes()).unwrap();
    assert!(!CodexParser::is_valid_file(temp_file3.path()));
}

#[tokio::test]
async fn test_codex_parser_parse() -> Result<()> {
    let mut temp_file = NamedTempFile::with_suffix(".jsonl").unwrap();
    let sample_data = r#"{"timestamp":"2025-10-12T14:10:16.717Z","type":"session_meta","payload":{"id":"550e8400-e29b-41d4-a716-446655440000","timestamp":"2024-01-01T10:00:00Z","cwd":"/Users/test/testproject","git":{"commit_hash":"abc123","branch":"main","repository_url":"git@github.com:user/test-project.git"}}}
{"timestamp":"2025-10-12T17:53:40.556Z","type":"event_msg","payload":{"type":"user_message","message":"Hello"}}
{"timestamp":"2025-10-12T17:53:43.040Z","type":"event_msg","payload":{"type":"agent_message","message":"Hi there!"}}"#;

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
    let sample_data = r#"{"timestamp":"2025-10-12T14:10:16.717Z","type":"session_meta","payload":{"id":"550e8400-e29b-41d4-a716-446655440000","timestamp":"2024-01-01T10:00:00Z"}}
{"timestamp":"2025-10-12T17:53:40.556Z","type":"event_msg","payload":{"type":"user_message","message":"Hello"}}"#;

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
    // Git-based project inference (no cwd, so falls back to git)
    let sample_data = r#"{"timestamp":"2025-10-12T14:10:16.717Z","type":"session_meta","payload":{"id":"550e8400-e29b-41d4-a716-446655440000","timestamp":"2024-01-01T10:00:00Z","git":{"commit_hash":"abc123","branch":"main","repository_url":"git@github.com:user/test-project.git"}}}
{"timestamp":"2025-10-12T17:53:40.556Z","type":"event_msg","payload":{"type":"user_message","message":"Hello"}}"#;

    temp_file.write_all(sample_data.as_bytes()).unwrap();

    let parser = CodexParser::new(temp_file.path());
    let result = parser.parse().await;

    assert!(result.is_ok());
    let (session, _messages) = result.unwrap();

    // Should have extracted project name from git URL (since no cwd)
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

    // CWD-based project inference
    let sample_data = r#"{"timestamp":"2025-10-12T14:10:16.717Z","type":"session_meta","payload":{"id":"550e8400-e29b-41d4-a716-446655440000","timestamp":"2024-01-01T10:00:00Z","cwd":"/Users/test/myproject"}}
{"timestamp":"2025-10-12T17:53:40.556Z","type":"event_msg","payload":{"type":"user_message","message":"Hello"}}"#;
    fs::write(&test_file, sample_data).unwrap();

    let parser = CodexParser::new(&test_file);
    let result = parser.parse().await;

    assert!(result.is_ok());
    let (session, _messages) = result.unwrap();

    // Should have inferred the project name from cwd (not file path)
    assert_eq!(session.project_name, Some("myproject".to_string()));
}

#[tokio::test]
async fn test_codex_parser_empty_content() -> Result<()> {
    let mut temp_file = NamedTempFile::with_suffix(".jsonl").unwrap();
    let sample_data = r#"{"timestamp":"2025-10-12T14:10:16.717Z","type":"session_meta","payload":{"id":"550e8400-e29b-41d4-a716-446655440000","timestamp":"2024-01-01T10:00:00Z"}}
{"timestamp":"2025-10-12T17:53:40.556Z","type":"event_msg","payload":{"type":"user_message","message":""}}"#;

    temp_file.write_all(sample_data.as_bytes()).unwrap();

    let parser = CodexParser::new(temp_file.path());
    let result = parser.parse().await;

    assert!(result.is_ok());
    let (session, messages) = result.unwrap();

    // Empty content should be discarded so 0
    assert_eq!(session.message_count, 0);

    Ok(())
}

#[tokio::test]
async fn test_codex_parser_skip_state_records() -> Result<()> {
    let mut temp_file = NamedTempFile::with_suffix(".jsonl").unwrap();
    // Include some non-message event types that should be ignored
    let sample_data = r#"{"timestamp":"2025-10-12T14:10:16.717Z","type":"session_meta","payload":{"id":"550e8400-e29b-41d4-a716-446655440000","timestamp":"2024-01-01T10:00:00Z"}}
{"timestamp":"2025-10-12T17:53:39.000Z","type":"other_event","payload":{}}
{"timestamp":"2025-10-12T17:53:39.500Z","type":"response_item","payload":{}}
{"timestamp":"2025-10-12T17:53:40.556Z","type":"event_msg","payload":{"type":"user_message","message":"Hello"}}"#;

    temp_file.write_all(sample_data.as_bytes()).unwrap();

    let parser = CodexParser::new(temp_file.path());
    let result = parser.parse().await;

    assert!(result.is_ok());
    let (session, messages) = result.unwrap();

    // Other event types should be skipped, only 1 message should be parsed
    assert_eq!(session.message_count, 1);
    assert_eq!(messages.len(), 1);

    Ok(())
}

#[tokio::test]
async fn test_codex_parser_invalid_uuid() {
    let mut temp_file = NamedTempFile::with_suffix(".jsonl").unwrap();
    let sample_data = r#"{"timestamp":"2025-10-12T14:10:16.717Z","type":"session_meta","payload":{"id":"invalid-uuid","timestamp":"2024-01-01T10:00:00Z"}}"#;

    temp_file.write_all(sample_data.as_bytes()).unwrap();

    let parser = CodexParser::new(temp_file.path());
    let result = parser.parse().await;

    assert!(result.is_err());
}

#[tokio::test]
async fn test_codex_parser_missing_header() {
    let mut temp_file = NamedTempFile::with_suffix(".jsonl").unwrap();
    // Missing session_meta, starts with message
    let sample_data = r#"{"timestamp":"2025-10-12T17:53:40.556Z","type":"event_msg","payload":{"type":"user_message","message":"Hello"}}"#;

    temp_file.write_all(sample_data.as_bytes()).unwrap();

    let parser = CodexParser::new(temp_file.path());
    let result = parser.parse().await;

    assert!(result.is_err());
}

#[tokio::test]
async fn test_codex_parser_file_consistency() {
    let mut temp_file = NamedTempFile::with_suffix(".jsonl").unwrap();
    let sample_data = r#"{"timestamp":"2025-10-12T14:10:16.717Z","type":"session_meta","payload":{"id":"550e8400-e29b-41d4-a716-446655440000","timestamp":"2024-01-01T10:00:00Z"}}
{"timestamp":"2025-10-12T17:53:40.556Z","type":"event_msg","payload":{"type":"user_message","message":"Hello"}}"#;

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
    // Multiple messages
    let sample_data = r#"{"timestamp":"2025-10-12T14:10:16.717Z","type":"session_meta","payload":{"id":"550e8400-e29b-41d4-a716-446655440000","timestamp":"2024-01-01T10:00:00Z"}}
{"timestamp":"2025-10-12T17:53:40.556Z","type":"event_msg","payload":{"type":"user_message","message":"Hello"}}
{"timestamp":"2025-10-12T17:53:41.000Z","type":"event_msg","payload":{"type":"user_message","message":"World"}}"#;

    temp_file.write_all(sample_data.as_bytes()).unwrap();

    let parser = CodexParser::new(temp_file.path());
    let result = parser.parse().await;

    assert!(result.is_ok());
    let (session, messages) = result.unwrap();

    assert_eq!(session.message_count, 2);
    assert_eq!(messages[0].content, "Hello");
    assert_eq!(messages[1].content, "World");

    Ok(())
}
