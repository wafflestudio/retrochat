use anyhow::Result;
use retrochat::models::Provider;
use retrochat::parsers::{CursorParser, ParserRegistry};
use std::fs;
use tempfile::TempDir;

fn create_cursor_test_structure(base_path: &std::path::Path) -> std::path::PathBuf {
    // Create multiple Cursor chat directories to simulate real structure
    let cursor_home = base_path.join(".cursor");
    let chats_dir = cursor_home.join("chats");

    // Create first chat
    let hash_dir1 = chats_dir.join("53460df9022de1a66445a5b78b067dd9");
    let uuid_dir1 = hash_dir1.join("557abc41-6f00-41e7-bf7b-696c80d4ee94");
    fs::create_dir_all(&uuid_dir1).unwrap();

    let store_db1 = uuid_dir1.join("store.db");
    create_test_database(
        &store_db1,
        "Chat Session 1",
        "557abc41-6f00-41e7-bf7b-696c80d4ee94",
    );

    // Create second chat
    let hash_dir2 = chats_dir.join("9f321443eb45496c4b77e9bc3bfefd29");
    let uuid_dir2 = hash_dir2.join("123e4567-e89b-12d3-a456-426614174000");
    fs::create_dir_all(&uuid_dir2).unwrap();

    let store_db2 = uuid_dir2.join("store.db");
    create_test_database(
        &store_db2,
        "Chat Session 2",
        "123e4567-e89b-12d3-a456-426614174000",
    );

    chats_dir
}

fn create_test_database(db_path: &std::path::Path, name: &str, agent_id: &str) {
    let conn = rusqlite::Connection::open(db_path).unwrap();

    // Create tables
    conn.execute("CREATE TABLE blobs (id TEXT PRIMARY KEY, data BLOB)", [])
        .unwrap();
    conn.execute("CREATE TABLE meta (key TEXT PRIMARY KEY, value TEXT)", [])
        .unwrap();

    // Create metadata
    let metadata = format!(
        r#"{{"agentId":"{agent_id}","latestRootBlobId":"d938807505e715cf66cef79253376e1294d65e8362bc76fb10cde93cc079d504","name":"{name}","mode":"default","createdAt":1758872189097,"lastUsedModel":"claude-3-5-sonnet"}}"#
    );
    let hex_metadata = hex::encode(metadata.as_bytes());

    conn.execute(
        "INSERT INTO meta (key, value) VALUES ('0', ?)",
        [&hex_metadata],
    )
    .unwrap();

    // Add test blob data with valid protobuf
    // Field 1 (0x0a): length-delimited string containing the name
    let mut blob_data = vec![0x0a, name.len() as u8]; // Field 1, length
    blob_data.extend_from_slice(name.as_bytes());

    conn.execute(
        "INSERT INTO blobs (id, data) VALUES ('test_blob', ?)",
        [&blob_data],
    )
    .unwrap();
}

#[tokio::test]
async fn test_cursor_parser_registry_detection() -> Result<()> {
    let temp_dir = TempDir::new().unwrap();
    let chats_dir = create_cursor_test_structure(temp_dir.path());

    // Test detection of Cursor store.db files
    let store_db1 = chats_dir
        .join("53460df9022de1a66445a5b78b067dd9")
        .join("557abc41-6f00-41e7-bf7b-696c80d4ee94")
        .join("store.db");

    let detected_provider = ParserRegistry::detect_provider(&store_db1);
    assert_eq!(detected_provider, Some(Provider::CursorAgent));

    // Test creation of parser
    let parser = ParserRegistry::create_parser(&store_db1)?;
    match parser {
        retrochat::parsers::ChatParser::Cursor(_) => {}
        _ => panic!("Expected Cursor parser"),
    }

    Ok(())
}

#[tokio::test]
async fn test_cursor_directory_scanning() -> Result<()> {
    let temp_dir = TempDir::new().unwrap();
    let chats_dir = create_cursor_test_structure(temp_dir.path());

    // Scan the chats directory recursively
    let found_files = ParserRegistry::scan_directory(&chats_dir, true, None)?;

    // Should find 2 Cursor store.db files
    let cursor_files: Vec<_> = found_files
        .iter()
        .filter(|(_, provider)| matches!(provider, Provider::CursorAgent))
        .collect();

    assert_eq!(cursor_files.len(), 2);

    // Verify file paths
    for (path, _) in &cursor_files {
        assert!(path.file_name().unwrap() == "store.db");
        assert!(path.to_string_lossy().contains("chats"));
    }

    Ok(())
}

#[tokio::test]
async fn test_cursor_parser_with_filter() -> Result<()> {
    let temp_dir = TempDir::new().unwrap();
    let chats_dir = create_cursor_test_structure(temp_dir.path());

    // Test scanning with Cursor filter only
    let cursor_filter = [Provider::CursorAgent];
    let found_files = ParserRegistry::scan_directory(&chats_dir, true, Some(&cursor_filter))?;

    assert_eq!(found_files.len(), 2);
    for (_, provider) in &found_files {
        assert_eq!(provider, &Provider::CursorAgent);
    }

    // Test scanning with different filter (should find nothing)
    let claude_filter = [Provider::ClaudeCode];
    let found_files = ParserRegistry::scan_directory(&chats_dir, true, Some(&claude_filter))?;
    assert_eq!(found_files.len(), 0);

    Ok(())
}

#[tokio::test]
async fn test_cursor_parse_multiple_sessions() -> Result<()> {
    let temp_dir = TempDir::new().unwrap();
    let chats_dir = create_cursor_test_structure(temp_dir.path());

    let found_files = ParserRegistry::scan_directory(&chats_dir, true, None)?;
    let cursor_files: Vec<_> = found_files
        .iter()
        .filter(|(_, provider)| matches!(provider, Provider::CursorAgent))
        .collect();

    assert_eq!(cursor_files.len(), 2);

    // Parse each file
    for (file_path, _) in cursor_files {
        let sessions = ParserRegistry::parse_file(file_path).await?;
        assert_eq!(sessions.len(), 1);

        let (session, messages) = &sessions[0];
        assert_eq!(session.provider, Provider::CursorAgent);
        assert_eq!(session.message_count, 1);
        assert_eq!(messages.len(), 1);
    }

    Ok(())
}

#[tokio::test]
async fn test_cursor_streaming_parse() -> Result<()> {
    let temp_dir = TempDir::new().unwrap();
    let chats_dir = create_cursor_test_structure(temp_dir.path());

    let store_db = chats_dir
        .join("53460df9022de1a66445a5b78b067dd9")
        .join("557abc41-6f00-41e7-bf7b-696c80d4ee94")
        .join("store.db");

    let mut session_count = 0;
    let mut message_count = 0;

    ParserRegistry::parse_file_streaming(&store_db, |session, message| {
        session_count += 1;
        message_count += 1;

        assert_eq!(session.provider, Provider::CursorAgent);
        assert!(message.content.contains("Chat Session 1"));

        Ok(())
    })
    .await?;

    assert_eq!(session_count, 1);
    assert_eq!(message_count, 1);

    Ok(())
}

#[test]
fn test_cursor_supported_extensions() {
    let extensions = ParserRegistry::get_supported_extensions();
    assert!(extensions.contains(&"db"));
}

#[test]
fn test_cursor_supported_providers() {
    let providers = ParserRegistry::get_supported_providers();
    assert!(providers.contains(&Provider::CursorAgent));
}

#[tokio::test]
async fn test_cursor_parser_with_project_inference() -> Result<()> {
    let temp_dir = TempDir::new().unwrap();

    // Create a directory structure that mimics a project
    let project_dir = temp_dir
        .path()
        .join("Users")
        .join("testuser")
        .join("Projects")
        .join("my-project");
    fs::create_dir_all(&project_dir).unwrap();

    // Create Cursor structure within the project
    let cursor_dir = project_dir.join(".cursor").join("chats");
    let hash_dir = cursor_dir.join("53460df9022de1a66445a5b78b067dd9");
    let uuid_dir = hash_dir.join("557abc41-6f00-41e7-bf7b-696c80d4ee94");
    fs::create_dir_all(&uuid_dir).unwrap();

    let store_db = uuid_dir.join("store.db");
    create_test_database(
        &store_db,
        "Project Chat",
        "557abc41-6f00-41e7-bf7b-696c80d4ee94",
    );

    let parser = CursorParser::new(&store_db);
    let (session, _) = parser.parse().await?;

    // Should extract project name from parent directory
    assert_eq!(session.project_name, Some("my-project".to_string()));

    Ok(())
}
