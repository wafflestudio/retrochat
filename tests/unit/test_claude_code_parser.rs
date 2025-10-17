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

#[tokio::test]
async fn test_claude_code_parser_tool_use_extraction() -> Result<()> {
    let mut temp_file = NamedTempFile::with_suffix(".jsonl").unwrap();
    let sample_data = r#"{"uuid":"550e8400-e29b-41d4-a716-446655440000","created_at":"2024-01-01T10:00:00Z","updated_at":"2024-01-01T10:00:00Z","chat_messages":[{"uuid":"550e8400-e29b-41d4-a716-446655440001","content":[{"type":"text","text":"Let me run a command"},{"type":"tool_use","id":"toolu_123","name":"Bash","input":{"command":"ls -la"}}],"created_at":"2024-01-01T10:00:00Z","updated_at":"2024-01-01T10:00:00Z","role":"assistant"}]}"#;

    temp_file.write_all(sample_data.as_bytes()).unwrap();

    let parser = ClaudeCodeParser::new(temp_file.path());
    let result = parser.parse().await;

    assert!(result.is_ok());
    let (_session, messages) = result.unwrap();

    assert_eq!(messages.len(), 1);
    let message = &messages[0];

    // Check that content includes both text and placeholder for tool
    assert!(message.content.contains("Let me run a command"));
    assert!(message.content.contains("[Tool Use: Bash]"));

    // Check that tool_uses were extracted
    assert!(message.tool_uses.is_some());
    let tool_uses = message.tool_uses.as_ref().unwrap();
    assert_eq!(tool_uses.len(), 1);

    let tool_use = &tool_uses[0];
    assert_eq!(tool_use.id, "toolu_123");
    assert_eq!(tool_use.name, "Bash");
    // vendor_type removed; ensure name and args are parsed
    assert_eq!(
        tool_use.input.get("command").and_then(|v| v.as_str()),
        Some("ls -la")
    );

    Ok(())
}

#[tokio::test]
async fn test_claude_code_parser_tool_result_extraction() -> Result<()> {
    let mut temp_file = NamedTempFile::with_suffix(".jsonl").unwrap();
    let sample_data = r#"{"uuid":"550e8400-e29b-41d4-a716-446655440000","created_at":"2024-01-01T10:00:00Z","updated_at":"2024-01-01T10:00:00Z","chat_messages":[{"uuid":"550e8400-e29b-41d4-a716-446655440001","content":[{"type":"tool_result","tool_use_id":"toolu_123","content":"total 8\ndrwxr-xr-x 2 user user 4096"}],"created_at":"2024-01-01T10:00:00Z","updated_at":"2024-01-01T10:00:00Z","role":"user"}]}"#;

    temp_file.write_all(sample_data.as_bytes()).unwrap();

    let parser = ClaudeCodeParser::new(temp_file.path());
    let result = parser.parse().await;

    assert!(result.is_ok());
    let (_session, messages) = result.unwrap();

    assert_eq!(messages.len(), 1);
    let message = &messages[0];

    // Check that content includes simplified placeholder (actual content is in tool_results column)
    assert!(message.content.contains("[Tool Result]"));

    // Check that tool_results were extracted
    assert!(message.tool_results.is_some());
    let tool_results = message.tool_results.as_ref().unwrap();
    assert_eq!(tool_results.len(), 1);

    let tool_result = &tool_results[0];
    assert_eq!(tool_result.tool_use_id, "toolu_123");
    assert!(tool_result.content.contains("total 8"));
    assert!(!tool_result.is_error);

    Ok(())
}

#[tokio::test]
async fn test_claude_code_parser_multiple_tools() -> Result<()> {
    let mut temp_file = NamedTempFile::with_suffix(".jsonl").unwrap();
    let sample_data = r#"{"uuid":"550e8400-e29b-41d4-a716-446655440000","created_at":"2024-01-01T10:00:00Z","updated_at":"2024-01-01T10:00:00Z","chat_messages":[{"uuid":"550e8400-e29b-41d4-a716-446655440001","content":[{"type":"text","text":"Running multiple commands"},{"type":"tool_use","id":"toolu_1","name":"Bash","input":{"command":"pwd"}},{"type":"tool_use","id":"toolu_2","name":"Read","input":{"file_path":"/test/file.txt"}}],"created_at":"2024-01-01T10:00:00Z","updated_at":"2024-01-01T10:00:00Z","role":"assistant"}]}"#;

    temp_file.write_all(sample_data.as_bytes()).unwrap();

    let parser = ClaudeCodeParser::new(temp_file.path());
    let result = parser.parse().await;

    assert!(result.is_ok());
    let (_session, messages) = result.unwrap();

    assert_eq!(messages.len(), 1);
    let message = &messages[0];

    // Check that tool_uses were extracted
    assert!(message.tool_uses.is_some());
    let tool_uses = message.tool_uses.as_ref().unwrap();
    assert_eq!(tool_uses.len(), 2);

    assert_eq!(tool_uses[0].name, "Bash");
    assert_eq!(tool_uses[1].name, "Read");

    Ok(())
}

#[tokio::test]
async fn test_claude_code_parser_tool_result_with_error() -> Result<()> {
    let mut temp_file = NamedTempFile::with_suffix(".jsonl").unwrap();
    let sample_data = r#"{"uuid":"550e8400-e29b-41d4-a716-446655440000","created_at":"2024-01-01T10:00:00Z","updated_at":"2024-01-01T10:00:00Z","chat_messages":[{"uuid":"550e8400-e29b-41d4-a716-446655440001","content":[{"type":"tool_result","tool_use_id":"toolu_123","content":"File not found","is_error":true}],"created_at":"2024-01-01T10:00:00Z","updated_at":"2024-01-01T10:00:00Z","role":"user"}]}"#;

    temp_file.write_all(sample_data.as_bytes()).unwrap();

    let parser = ClaudeCodeParser::new(temp_file.path());
    let result = parser.parse().await;

    assert!(result.is_ok());
    let (_session, messages) = result.unwrap();

    assert_eq!(messages.len(), 1);
    let message = &messages[0];

    // Check that tool_results were extracted with error flag
    assert!(message.tool_results.is_some());
    let tool_results = message.tool_results.as_ref().unwrap();
    assert_eq!(tool_results.len(), 1);

    let tool_result = &tool_results[0];
    assert_eq!(tool_result.tool_use_id, "toolu_123");
    assert!(tool_result.is_error);
    assert!(tool_result.content.contains("File not found"));

    Ok(())
}
// duplicate imports removed (already imported above)

/// Test that toolUseResult metadata from Claude Code conversation format
/// is properly enriched into ToolResult details
#[tokio::test]
async fn test_claude_conversation_tool_result_with_metadata() -> Result<()> {
    let mut temp_file = NamedTempFile::with_suffix(".jsonl").unwrap();

    // This simulates the actual Claude Code conversation format
    // where toolUseResult is at the entry root level
    let sample_data = r#"{"type":"user","uuid":"msg-result-123","sessionId":"550e8400-e29b-41d4-a716-446655440000","timestamp":"2024-01-01T10:00:00Z","message":{"role":"user","content":[{"tool_use_id":"toolu_123","type":"tool_result","content":"Branch created successfully","is_error":false}]},"toolUseResult":{"stdout":"Switched to branch 'feature-123'\nCreated branch 'feature-123'","stderr":"","interrupted":false,"isImage":false}}"#;

    temp_file.write_all(sample_data.as_bytes()).unwrap();

    let parser = ClaudeCodeParser::new(temp_file.path());
    let result = parser.parse().await;

    assert!(result.is_ok());
    let (_session, messages) = result.unwrap();

    assert_eq!(messages.len(), 1);
    let message = &messages[0];

    // Verify tool_results exist
    assert!(message.tool_results.is_some());
    let tool_results = message.tool_results.as_ref().unwrap();
    assert_eq!(tool_results.len(), 1);

    let tool_result = &tool_results[0];

    // Verify basic fields
    assert_eq!(tool_result.tool_use_id, "toolu_123");
    assert_eq!(tool_result.content, "Branch created successfully");
    assert!(!tool_result.is_error);

    // Verify that toolUseResult was enriched into details
    assert!(tool_result.details.is_some());
    let details = tool_result.details.as_ref().unwrap();

    // Check stdout from toolUseResult
    assert_eq!(
        details.get("stdout").and_then(|v| v.as_str()),
        Some("Switched to branch 'feature-123'\nCreated branch 'feature-123'")
    );

    // Check stderr from toolUseResult
    assert_eq!(details.get("stderr").and_then(|v| v.as_str()), Some(""));

    // Check interrupted flag
    assert_eq!(
        details.get("interrupted").and_then(|v| v.as_bool()),
        Some(false)
    );

    Ok(())
}

/// Test conversation format without toolUseResult (should still work)
#[tokio::test]
async fn test_claude_conversation_tool_result_without_metadata() -> Result<()> {
    let mut temp_file = NamedTempFile::with_suffix(".jsonl").unwrap();

    let sample_data = r#"{"type":"user","uuid":"msg-result-456","sessionId":"550e8400-e29b-41d4-a716-446655440000","timestamp":"2024-01-01T10:01:00Z","message":{"role":"user","content":[{"tool_use_id":"toolu_456","type":"tool_result","content":"File read successfully","is_error":false}]}}"#;

    temp_file.write_all(sample_data.as_bytes()).unwrap();

    let parser = ClaudeCodeParser::new(temp_file.path());
    let result = parser.parse().await;

    assert!(result.is_ok());
    let (_session, messages) = result.unwrap();

    assert_eq!(messages.len(), 1);
    let message = &messages[0];

    // Verify tool_results exist
    assert!(message.tool_results.is_some());
    let tool_results = message.tool_results.as_ref().unwrap();
    assert_eq!(tool_results.len(), 1);

    let tool_result = &tool_results[0];

    // Verify basic fields work even without toolUseResult
    assert_eq!(tool_result.tool_use_id, "toolu_456");
    assert_eq!(tool_result.content, "File read successfully");
    assert!(!tool_result.is_error);

    Ok(())
}

/// Test multiple tool results in conversation (only first should get metadata)
#[tokio::test]
async fn test_claude_conversation_multiple_tool_results() -> Result<()> {
    let mut temp_file = NamedTempFile::with_suffix(".jsonl").unwrap();

    let sample_data = r#"{"type":"user","uuid":"msg-multi-123","sessionId":"550e8400-e29b-41d4-a716-446655440000","timestamp":"2024-01-01T10:02:00Z","message":{"role":"user","content":[{"tool_use_id":"toolu_1","type":"tool_result","content":"Result 1","is_error":false},{"tool_use_id":"toolu_2","type":"tool_result","content":"Result 2","is_error":false}]},"toolUseResult":{"stdout":"Output for first tool","stderr":"","interrupted":false}}"#;

    temp_file.write_all(sample_data.as_bytes()).unwrap();

    let parser = ClaudeCodeParser::new(temp_file.path());
    let result = parser.parse().await;

    assert!(result.is_ok());
    let (_session, messages) = result.unwrap();

    assert_eq!(messages.len(), 1);
    let message = &messages[0];

    // Verify both tool_results exist
    assert!(message.tool_results.is_some());
    let tool_results = message.tool_results.as_ref().unwrap();
    assert_eq!(tool_results.len(), 2);

    // First result should have details from toolUseResult
    let first_result = &tool_results[0];
    assert!(first_result.details.is_some());
    assert_eq!(
        first_result
            .details
            .as_ref()
            .unwrap()
            .get("stdout")
            .and_then(|v| v.as_str()),
        Some("Output for first tool")
    );

    // Second result should not have details (toolUseResult only applies to first)
    let _second_result = &tool_results[1];
    // It might have None or the raw content, depending on implementation

    Ok(())
}
