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

#[tokio::test]
async fn test_gemini_parser_filename_session_id() -> Result<()> {
    let temp_dir = TempDir::new().unwrap();
    let base_path = temp_dir.path();

    // Create a file with session-*.json pattern
    let test_file = base_path.join("session-2025-10-12T17-51-4b2d82b4.json");

    // Create an array of messages (new format based on user's description)
    let sample_data = r#"[
        {"id":"msg-1","timestamp":"2024-01-01T10:00:00Z","type":"user","content":"Hello","tokens":{"input":5,"output":0,"cached":0,"thoughts":0,"tool":0,"total":5}},
        {"id":"msg-2","timestamp":"2024-01-01T10:01:00Z","type":"gemini","content":"Hi there! How can I help you?","tokens":{"input":0,"output":20,"cached":0,"thoughts":0,"tool":0,"total":20}}
    ]"#;

    fs::write(&test_file, sample_data).unwrap();

    let parser = GeminiCLIParser::new(&test_file);
    let result = parser.parse().await;

    assert!(result.is_ok());
    let sessions = result.unwrap();

    assert_eq!(sessions.len(), 1);

    let (session, messages) = &sessions[0];
    assert_eq!(session.provider, Provider::GeminiCLI);
    assert_eq!(session.message_count, 2);
    assert_eq!(messages.len(), 2);

    // Check that messages were parsed correctly
    assert_eq!(messages[0].content, "Hello");
    assert_eq!(messages[1].content, "Hi there! How can I help you?");

    // Check token counts
    assert_eq!(messages[0].token_count, Some(5));
    assert_eq!(messages[1].token_count, Some(20));
    assert_eq!(session.token_count, Some(25));

    // Check that project name was inferred from the filename
    // The last part after the last hyphen should be used as project identifier
    assert_eq!(session.project_name, Some("4b2d82b4".to_string()));

    Ok(())
}

#[tokio::test]
async fn test_gemini_parser_tool_operations() -> Result<()> {
    let temp_dir = TempDir::new().unwrap();
    let base_path = temp_dir.path();

    // Create a file with session-*.json pattern
    let test_file = base_path.join("session-2025-10-23T08-55-test.json");

    // Create sample data with tool calls
    // Note: Using concat to avoid raw string escaping issues
    let sample_data = r#"[{"id":"msg-1","timestamp":"2024-01-01T10:00:00Z","type":"user","content":"Please edit the file"},{"id":"msg-2","timestamp":"2024-01-01T10:01:00Z","type":"gemini","content":"edit","toolCalls":[{"id":"replace-123","name":"replace","args":{"old_string":"hello","new_string":"goodbye","file_path":"/test/file.txt"},"result":[{"functionResponse":{"id":"replace-123","name":"replace","response":{"output":"Successfully modified file: /test/file.txt"}}}],"status":"success","timestamp":"2024-01-01T10:01:30Z"}]},{"id":"msg-3","timestamp":"2024-01-01T10:02:00Z","type":"gemini","content":"bash","toolCalls":[{"id":"bash-456","name":"run_shell_command","args":{"command":"ls -la"},"result":[{"functionResponse":{"id":"bash-456","name":"run_shell_command","response":{"output":"total 0"}}}],"status":"success","timestamp":"2024-01-01T10:02:30Z"}]},{"id":"msg-4","timestamp":"2024-01-01T10:03:00Z","type":"gemini","content":"read","toolCalls":[{"id":"read-789","name":"read_file","args":{"file_path":"/test/config.json"},"result":[{"functionResponse":{"id":"read-789","name":"read_file","response":{"output":"config data"}}}],"status":"success","timestamp":"2024-01-01T10:03:30Z"}]}]"#;

    fs::write(&test_file, sample_data).unwrap();

    let parser = GeminiCLIParser::new(&test_file);
    let result = parser.parse().await;

    if let Err(e) = &result {
        eprintln!("Parse error: {e:?}");
    }
    assert!(result.is_ok());
    let sessions = result.unwrap();

    assert_eq!(sessions.len(), 1);

    let (session, messages) = &sessions[0];
    assert_eq!(session.provider, Provider::GeminiCLI);
    assert_eq!(session.message_count, 4);
    assert_eq!(messages.len(), 4);

    // Check first message has no tools
    assert!(messages[0].tool_uses.is_none());
    assert!(messages[0].tool_results.is_none());

    // Check second message has Edit tool
    let msg2_tool_uses = messages[1].tool_uses.as_ref().unwrap();
    let msg2_tool_results = messages[1].tool_results.as_ref().unwrap();
    assert_eq!(msg2_tool_uses.len(), 1);
    assert_eq!(msg2_tool_results.len(), 1);
    assert_eq!(msg2_tool_uses[0].name, "Edit"); // Normalized from "replace"
    assert_eq!(msg2_tool_uses[0].id, "replace-123");
    assert_eq!(msg2_tool_results[0].tool_use_id, "replace-123");
    assert!(!msg2_tool_results[0].is_error);
    assert!(msg2_tool_results[0]
        .content
        .contains("Successfully modified"));

    // Check third message has Bash tool
    let msg3_tool_uses = messages[2].tool_uses.as_ref().unwrap();
    let msg3_tool_results = messages[2].tool_results.as_ref().unwrap();
    assert_eq!(msg3_tool_uses.len(), 1);
    assert_eq!(msg3_tool_results.len(), 1);
    assert_eq!(msg3_tool_uses[0].name, "Bash"); // Normalized from "run_shell_command"
    assert_eq!(msg3_tool_uses[0].id, "bash-456");
    assert!(msg3_tool_results[0].content.contains("total"));

    // Check fourth message has Read tool
    let msg4_tool_uses = messages[3].tool_uses.as_ref().unwrap();
    let msg4_tool_results = messages[3].tool_results.as_ref().unwrap();
    assert_eq!(msg4_tool_uses.len(), 1);
    assert_eq!(msg4_tool_results.len(), 1);
    assert_eq!(msg4_tool_uses[0].name, "Read"); // Normalized from "read_file"
    assert_eq!(msg4_tool_uses[0].id, "read-789");

    Ok(())
}

#[tokio::test]
async fn test_gemini_parser_tool_name_normalization() -> Result<()> {
    let temp_dir = TempDir::new().unwrap();
    let base_path = temp_dir.path();
    let test_file = base_path.join("session-test-tool-norm.json");

    // Test various tool names get normalized correctly
    let sample_data = r#"[
        {"id":"msg-1","timestamp":"2024-01-01T10:00:00Z","type":"gemini","content":"Tools","toolCalls":[
            {"id":"1","name":"replace","args":{},"status":"success"},
            {"id":"2","name":"run_shell_command","args":{},"status":"success"},
            {"id":"3","name":"read_file","args":{},"status":"success"},
            {"id":"4","name":"write_file","args":{},"status":"success"},
            {"id":"5","name":"write_to_file","args":{},"status":"success"},
            {"id":"6","name":"some_unknown_tool","args":{},"status":"success"}
        ]}
    ]"#;

    fs::write(&test_file, sample_data).unwrap();

    let parser = GeminiCLIParser::new(&test_file);
    let result = parser.parse().await;

    assert!(result.is_ok());
    let sessions = result.unwrap();
    let (_, messages) = &sessions[0];

    let tool_uses = messages[0].tool_uses.as_ref().unwrap();
    assert_eq!(tool_uses.len(), 6);

    // Check name normalization
    assert_eq!(tool_uses[0].name, "Edit");
    assert_eq!(tool_uses[1].name, "Bash");
    assert_eq!(tool_uses[2].name, "Read");
    assert_eq!(tool_uses[3].name, "Write");
    assert_eq!(tool_uses[4].name, "Write");
    assert_eq!(tool_uses[5].name, "Some_unknown_tool"); // Capitalized

    Ok(())
}

#[tokio::test]
async fn test_gemini_parser_tool_error_status() -> Result<()> {
    let temp_dir = TempDir::new().unwrap();
    let base_path = temp_dir.path();
    let test_file = base_path.join("session-test-tool-error.json");

    // Test error status detection
    let sample_data = r#"[
        {"id":"msg-1","timestamp":"2024-01-01T10:00:00Z","type":"gemini","content":"Error test","toolCalls":[
            {
                "id":"err-1","name":"replace","args":{},"status":"failed",
                "result":[{"functionResponse":{"id":"err-1","name":"replace","response":{"output":"Error: file not found"}}}]
            },
            {
                "id":"ok-1","name":"replace","args":{},"status":"success",
                "result":[{"functionResponse":{"id":"ok-1","name":"replace","response":{"output":"Success"}}}]
            }
        ]}
    ]"#;

    fs::write(&test_file, sample_data).unwrap();

    let parser = GeminiCLIParser::new(&test_file);
    let result = parser.parse().await;

    assert!(result.is_ok());
    let sessions = result.unwrap();
    let (_, messages) = &sessions[0];

    let tool_results = messages[0].tool_results.as_ref().unwrap();
    assert_eq!(tool_results.len(), 2);

    // First result should be marked as error
    assert!(tool_results[0].is_error);
    assert!(tool_results[0].content.contains("Error"));

    // Second result should be marked as success
    assert!(!tool_results[1].is_error);
    assert!(tool_results[1].content.contains("Success"));

    Ok(())
}
