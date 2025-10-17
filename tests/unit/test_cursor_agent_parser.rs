use anyhow::Result;
use retrochat::models::Provider;
use retrochat::parsers::CursorAgentParser;
use std::fs;
use tempfile::TempDir;

fn create_cursor_test_database(base_path: &std::path::Path) -> std::path::PathBuf {
    // Create Cursor directory structure
    let chats_dir = base_path.join("chats");
    let hash_dir = chats_dir.join("53460df9022de1a66445a5b78b067dd9");
    let uuid_dir = hash_dir.join("557abc41-6f00-41e7-bf7b-696c80d4ee94");
    fs::create_dir_all(&uuid_dir).unwrap();

    let store_db = uuid_dir.join("store.db");

    // Create a simple SQLite database with test data
    let conn = rusqlite::Connection::open(&store_db).unwrap();

    // Create tables
    conn.execute("CREATE TABLE blobs (id TEXT PRIMARY KEY, data BLOB)", [])
        .unwrap();

    conn.execute("CREATE TABLE meta (key TEXT PRIMARY KEY, value TEXT)", [])
        .unwrap();

    // Insert test metadata (hex-encoded JSON)
    let test_metadata = r#"{"agentId":"557abc41-6f00-41e7-bf7b-696c80d4ee94","latestRootBlobId":"d938807505e715cf66cef79253376e1294d65e8362bc76fb10cde93cc079d504","name":"Test Chat Session","mode":"default","createdAt":1758872189097,"lastUsedModel":"claude-3-5-sonnet"}"#;
    let hex_metadata = hex::encode(test_metadata.as_bytes());

    conn.execute(
        "INSERT INTO meta (key, value) VALUES ('0', ?)",
        [&hex_metadata],
    )
    .unwrap();

    // Insert test blob with valid protobuf data
    // Field 1 (0x0a): length-delimited string "Hello from test"
    let test_message = b"Hello from test";
    let mut blob_data = vec![0x0a, test_message.len() as u8]; // Field 1, length
    blob_data.extend_from_slice(test_message);

    conn.execute(
        "INSERT INTO blobs (id, data) VALUES ('test_blob_id', ?)",
        [&blob_data],
    )
    .unwrap();

    store_db
}

#[test]
fn test_cursor_parser_is_valid_file() {
    let temp_dir = TempDir::new().unwrap();
    let store_db = create_cursor_test_database(temp_dir.path());

    assert!(CursorAgentParser::is_valid_file(&store_db));
}

#[test]
fn test_cursor_parser_is_invalid_file() {
    let temp_dir = TempDir::new().unwrap();

    // Test invalid file name
    let invalid_file = temp_dir.path().join("not_store.db");
    fs::write(&invalid_file, "").unwrap();
    assert!(!CursorAgentParser::is_valid_file(&invalid_file));

    // Test wrong directory structure
    let wrong_structure = temp_dir.path().join("wrong").join("store.db");
    fs::create_dir_all(wrong_structure.parent().unwrap()).unwrap();
    fs::write(&wrong_structure, "").unwrap();
    assert!(!CursorAgentParser::is_valid_file(&wrong_structure));
}

#[tokio::test]
async fn test_cursor_parser_parse() -> Result<()> {
    let temp_dir = TempDir::new().unwrap();
    let store_db = create_cursor_test_database(temp_dir.path());

    let parser = CursorAgentParser::new(&store_db);
    let result = parser.parse().await?;

    let (session, messages) = result;

    // Verify session properties
    assert_eq!(session.provider, Provider::CursorAgent);
    assert_eq!(
        session.id.to_string(),
        "557abc41-6f00-41e7-bf7b-696c80d4ee94"
    );
    assert_eq!(session.message_count, 1);

    // Verify we have the parsed message
    assert_eq!(messages.len(), 1);
    assert!(messages[0].content.contains("Hello from test"));

    Ok(())
}

#[tokio::test]
async fn test_cursor_parser_parse_streaming() -> Result<()> {
    let temp_dir = TempDir::new().unwrap();
    let store_db = create_cursor_test_database(temp_dir.path());

    let parser = CursorAgentParser::new(&store_db);

    let mut session_count = 0;
    let mut message_count = 0;

    parser
        .parse_streaming(|session, _message| {
            session_count += 1;
            message_count += 1;

            assert_eq!(session.provider, Provider::CursorAgent);
            // Role can be User or Assistant based on heuristics

            Ok(())
        })
        .await?;

    assert_eq!(session_count, 1);
    assert_eq!(message_count, 1);

    Ok(())
}

#[tokio::test]
async fn test_cursor_parser_metadata_extraction() {
    let temp_dir = TempDir::new().unwrap();
    let store_db = create_cursor_test_database(temp_dir.path());

    let parser = CursorAgentParser::new(&store_db);
    let (session, _) = parser.parse().await.unwrap();

    // Verify that metadata was correctly extracted and used
    assert_eq!(
        session.id.to_string(),
        "557abc41-6f00-41e7-bf7b-696c80d4ee94"
    );
    assert_eq!(session.provider, Provider::CursorAgent);
}

#[tokio::test]
async fn test_cursor_parser_timestamp_handling() {
    let temp_dir = TempDir::new().unwrap();
    let store_db = create_cursor_test_database(temp_dir.path());

    let parser = CursorAgentParser::new(&store_db);
    let (session, _) = parser.parse().await.unwrap();

    // Should have converted timestamp correctly (1758872189097 ms -> valid DateTime)
    assert!(session.start_time.timestamp() > 0);
}

#[tokio::test]
async fn test_cursor_parser_file_consistency() {
    let temp_dir = TempDir::new().unwrap();
    let store_db = create_cursor_test_database(temp_dir.path());

    let parser1 = CursorAgentParser::new(&store_db);
    let parser2 = CursorAgentParser::new(&store_db);

    let (session1, _) = parser1.parse().await.unwrap();
    let (session2, _) = parser2.parse().await.unwrap();

    // File hash should be consistent between parsers
    assert_eq!(session1.file_hash, session2.file_hash);
    assert!(!session1.file_hash.is_empty());
}

#[tokio::test]
async fn test_cursor_parser_invalid_database() {
    let temp_dir = TempDir::new().unwrap();
    let invalid_db = temp_dir
        .path()
        .join("chats")
        .join("invalidhash")
        .join("557abc41-6f00-41e7-bf7b-696c80d4ee94")
        .join("store.db");

    fs::create_dir_all(invalid_db.parent().unwrap()).unwrap();
    fs::write(&invalid_db, "not a database").unwrap();

    let parser = CursorAgentParser::new(&invalid_db);
    let result = parser.parse().await;

    assert!(result.is_err());
}

#[tokio::test]
async fn test_cursor_parser_missing_metadata() {
    let temp_dir = TempDir::new().unwrap();
    let base_path = temp_dir.path();

    // Create directory structure
    let chats_dir = base_path.join("chats");
    let hash_dir = chats_dir.join("53460df9022de1a66445a5b78b067dd9");
    let uuid_dir = hash_dir.join("557abc41-6f00-41e7-bf7b-696c80d4ee94");
    fs::create_dir_all(&uuid_dir).unwrap();

    let store_db = uuid_dir.join("store.db");

    // Create database without metadata
    let conn = rusqlite::Connection::open(&store_db).unwrap();
    conn.execute("CREATE TABLE meta (key TEXT PRIMARY KEY, value TEXT)", [])
        .unwrap();
    // Don't insert any metadata

    let parser = CursorAgentParser::new(&store_db);
    let result = parser.parse().await;

    assert!(result.is_err());
}

#[tokio::test]
async fn test_cursor_parser_tool_call_extraction() -> Result<()> {
    let temp_dir = TempDir::new().unwrap();
    let base_path = temp_dir.path();

    // Create Cursor directory structure
    let chats_dir = base_path.join("chats");
    let hash_dir = chats_dir.join("53460df9022de1a66445a5b78b067dd9");
    let uuid_dir = hash_dir.join("557abc41-6f00-41e7-bf7b-696c80d4ee94");
    fs::create_dir_all(&uuid_dir).unwrap();

    let store_db = uuid_dir.join("store.db");

    // Create database
    let conn = rusqlite::Connection::open(&store_db).unwrap();
    conn.execute("CREATE TABLE blobs (id TEXT PRIMARY KEY, data BLOB)", [])
        .unwrap();
    conn.execute("CREATE TABLE meta (key TEXT PRIMARY KEY, value TEXT)", [])
        .unwrap();

    // Insert metadata
    let test_metadata = r#"{"agentId":"557abc41-6f00-41e7-bf7b-696c80d4ee94","latestRootBlobId":"test","name":"Test","mode":"default","createdAt":1758872189097,"lastUsedModel":"claude-3-5-sonnet"}"#;
    let hex_metadata = hex::encode(test_metadata.as_bytes());
    conn.execute(
        "INSERT INTO meta (key, value) VALUES ('0', ?)",
        [&hex_metadata],
    )
    .unwrap();

    // Create blob with JSON that includes tool-call
    let json_content = r#"{"role":"assistant","content":[{"type":"text","text":"Running a command"},{"type":"tool-call","toolName":"Bash","args":{"command":"ls -la"}}]}"#;

    // Encode as field 4 (protobuf field number 4, wire type 2 = length-delimited)
    let field_key = (4 << 3) | 2; // Field 4, wire type 2
    let mut blob_data = vec![field_key as u8];

    // Properly encode length as varint
    let len = json_content.len();
    if len < 128 {
        blob_data.push(len as u8);
    } else {
        // Two-byte varint for lengths 128-16383
        blob_data.push(((len & 0x7f) | 0x80) as u8);
        blob_data.push((len >> 7) as u8);
    }

    blob_data.extend_from_slice(json_content.as_bytes());

    conn.execute(
        "INSERT INTO blobs (id, data) VALUES ('test_blob', ?)",
        [&blob_data],
    )
    .unwrap();

    let parser = CursorAgentParser::new(&store_db);
    let (_session, messages) = parser.parse().await?;

    assert_eq!(messages.len(), 1);
    let message = &messages[0];

    // Check that content includes text and tool placeholder
    assert!(message.content.contains("Running a command"));
    assert!(message.content.contains("[Tool: Bash]"));

    // Check that tool_uses were extracted
    assert!(message.tool_uses.is_some());
    let tool_uses = message.tool_uses.as_ref().unwrap();
    assert_eq!(tool_uses.len(), 1);

    let tool_use = &tool_uses[0];
    assert!(tool_use.id.starts_with("test_blob-tool-"));
    assert_eq!(tool_use.name, "Bash");
    // vendor_type removed; ensure name and args are parsed
    assert_eq!(
        tool_use.input.get("command").and_then(|v| v.as_str()),
        Some("ls -la")
    );

    Ok(())
}

#[tokio::test]
async fn test_cursor_parser_multiple_tool_calls() -> Result<()> {
    let temp_dir = TempDir::new().unwrap();
    let base_path = temp_dir.path();

    let chats_dir = base_path.join("chats");
    let hash_dir = chats_dir.join("53460df9022de1a66445a5b78b067dd9");
    let uuid_dir = hash_dir.join("557abc41-6f00-41e7-bf7b-696c80d4ee94");
    fs::create_dir_all(&uuid_dir).unwrap();

    let store_db = uuid_dir.join("store.db");

    let conn = rusqlite::Connection::open(&store_db).unwrap();
    conn.execute("CREATE TABLE blobs (id TEXT PRIMARY KEY, data BLOB)", [])
        .unwrap();
    conn.execute("CREATE TABLE meta (key TEXT PRIMARY KEY, value TEXT)", [])
        .unwrap();

    let test_metadata = r#"{"agentId":"557abc41-6f00-41e7-bf7b-696c80d4ee94","latestRootBlobId":"test","name":"Test","mode":"default","createdAt":1758872189097,"lastUsedModel":"claude-3-5-sonnet"}"#;
    let hex_metadata = hex::encode(test_metadata.as_bytes());
    conn.execute(
        "INSERT INTO meta (key, value) VALUES ('0', ?)",
        [&hex_metadata],
    )
    .unwrap();

    // Create blob with multiple tool-calls
    let json_content = r#"{"role":"assistant","content":[{"type":"text","text":"Running commands"},{"type":"tool-call","toolName":"Bash","args":{"command":"pwd"}},{"type":"tool-call","toolName":"Read","args":{"file_path":"/test/file"}}]}"#;

    let field_key = (4 << 3) | 2;
    let mut blob_data = vec![field_key as u8];

    // Properly encode length as varint
    let len = json_content.len();
    if len < 128 {
        blob_data.push(len as u8);
    } else {
        // Two-byte varint for lengths 128-16383
        blob_data.push(((len & 0x7f) | 0x80) as u8);
        blob_data.push((len >> 7) as u8);
    }

    blob_data.extend_from_slice(json_content.as_bytes());

    conn.execute(
        "INSERT INTO blobs (id, data) VALUES ('multi_tool', ?)",
        [&blob_data],
    )
    .unwrap();

    let parser = CursorAgentParser::new(&store_db);
    let (_session, messages) = parser.parse().await?;

    assert_eq!(messages.len(), 1);
    let message = &messages[0];

    // Check that tool_uses were extracted
    assert!(message.tool_uses.is_some());
    let tool_uses = message.tool_uses.as_ref().unwrap();
    assert_eq!(tool_uses.len(), 2);

    assert_eq!(tool_uses[0].name, "Bash");
    assert_eq!(tool_uses[0].id, "multi_tool-tool-0");

    assert_eq!(tool_uses[1].name, "Read");
    assert_eq!(tool_uses[1].id, "multi_tool-tool-1");

    Ok(())
}

#[tokio::test]
async fn test_cursor_parser_no_tools() -> Result<()> {
    let temp_dir = TempDir::new().unwrap();
    let store_db = create_cursor_test_database(temp_dir.path());

    let parser = CursorAgentParser::new(&store_db);
    let (_session, messages) = parser.parse().await?;

    assert_eq!(messages.len(), 1);
    let message = &messages[0];

    // Should not have any tools
    assert!(message.tool_uses.is_none() || message.tool_uses.as_ref().unwrap().is_empty());
    assert!(message.tool_results.is_none() || message.tool_results.as_ref().unwrap().is_empty());

    Ok(())
}
