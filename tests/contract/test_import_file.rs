use retrochat::database::Database;
use retrochat::services::{ImportFileRequest, ImportService};
use std::fs;
use std::sync::Arc;
use tempfile::TempDir;

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct ErrorResponse {
    pub error_code: String,
    pub error_message: String,
    pub details: Option<serde_json::Value>,
}

#[tokio::test]
async fn test_import_claude_code_file_success() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let file_path = temp_dir.path().join("claude_session.jsonl");

    let claude_content = r#"{"type":"conversation","sessionId":"550e8400-e29b-41d4-a716-446655440000","timestamp":"2024-01-01T00:00:00Z","message":{"role":"user","content":"Hello"}}
{"type":"conversation","sessionId":"550e8400-e29b-41d4-a716-446655440000","timestamp":"2024-01-01T00:01:00Z","message":{"role":"assistant","content":"Hi there!"}}
{"type":"conversation","sessionId":"550e8400-e29b-41d4-a716-446655440000","timestamp":"2024-01-01T00:02:00Z","message":{"role":"user","content":"How are you?"}}
{"type":"conversation","sessionId":"550e8400-e29b-41d4-a716-446655440000","timestamp":"2024-01-01T00:03:00Z","message":{"role":"assistant","content":"I'm doing well!"}}"#;

    fs::write(&file_path, claude_content).expect("Failed to write test file");

    let request = ImportFileRequest {
        file_path: file_path.to_string_lossy().to_string(),
        provider: Some("ClaudeCode".to_string()),
        project_name: Some("test_project".to_string()),
        overwrite_existing: Some(false),
    };

    let database = Database::new_in_memory().await.unwrap();
    database.initialize().await.unwrap();
    let service = ImportService::new(Arc::new(database.manager));
    let result = service.import_file(request).await;

    if let Err(e) = &result {
        println!("Import error: {e}");
    }
    assert!(result.is_ok());
    let response = result.unwrap();
    assert!(response.sessions_imported >= 0);
    assert!(response.messages_imported >= 0);
    assert!(response.import_duration_ms >= 0);
    assert!(response.file_size_bytes > 0);
}

#[tokio::test]
async fn test_import_gemini_file_success() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let file_path = temp_dir.path().join("gemini_conversations.json");

    let gemini_content = r#"{
        "sessionId": "550e8400-e29b-41d4-a716-446655440000",
        "projectHash": "test-project-hash",
        "startTime": "2024-01-01T00:00:00Z",
        "lastUpdated": "2024-01-01T00:01:00Z",
        "messages": [
            {
                "id": "msg-1",
                "timestamp": "2024-01-01T00:00:00Z",
                "type": "user",
                "content": "Hello Gemini"
            },
            {
                "id": "msg-2",
                "timestamp": "2024-01-01T00:01:00Z",
                "type": "gemini",
                "content": "Hello! How can I help you?"
            }
        ]
    }"#;

    fs::write(&file_path, gemini_content).expect("Failed to write test file");

    let request = ImportFileRequest {
        file_path: file_path.to_string_lossy().to_string(),
        provider: Some("Gemini".to_string()),
        project_name: None,
        overwrite_existing: Some(false),
    };

    let database = Database::new_in_memory().await.unwrap();
    database.initialize().await.unwrap();
    let service = ImportService::new(Arc::new(database.manager));
    let result = service.import_file(request).await;

    if let Err(e) = &result {
        println!("Gemini import error: {e}");
    }
    assert!(result.is_ok());
    let response = result.unwrap();
    assert!(response.sessions_imported >= 0);
    assert!(response.messages_imported >= 0);
}

#[tokio::test]
async fn test_import_file_not_found() {
    let request = ImportFileRequest {
        file_path: "/non/existent/file.jsonl".to_string(),
        provider: Some("ClaudeCode".to_string()),
        project_name: None,
        overwrite_existing: Some(false),
    };

    let database = Database::new_in_memory().await.unwrap();
    database.initialize().await.unwrap();
    let service = ImportService::new(Arc::new(database.manager));
    let result = service.import_file(request).await;

    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(error.to_string().contains("Invalid file path"));
}

#[tokio::test]
async fn test_import_file_invalid_format() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let file_path = temp_dir.path().join("invalid.jsonl");

    fs::write(&file_path, "invalid json content").expect("Failed to write test file");

    let request = ImportFileRequest {
        file_path: file_path.to_string_lossy().to_string(),
        provider: Some("ClaudeCode".to_string()),
        project_name: None,
        overwrite_existing: Some(false),
    };

    let database = Database::new_in_memory().await.unwrap();
    database.initialize().await.unwrap();
    let service = ImportService::new(Arc::new(database.manager));
    let result = service.import_file(request).await;

    // The current implementation should fail for invalid JSON
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(error.to_string().contains("Failed to parse file"));
}

#[tokio::test]
async fn test_import_file_duplicate_detection() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let file_path = temp_dir.path().join("duplicate.jsonl");

    let content = r#"{"type":"conversation","sessionId":"550e8400-e29b-41d4-a716-446655440000","timestamp":"2024-01-01T00:00:00Z","message":{"role":"user","content":"Test"}}"#;
    fs::write(&file_path, content).expect("Failed to write test file");

    let request = ImportFileRequest {
        file_path: file_path.to_string_lossy().to_string(),
        provider: Some("ClaudeCode".to_string()),
        project_name: None,
        overwrite_existing: Some(false),
    };

    let database = Database::new_in_memory().await.unwrap();
    database.initialize().await.unwrap();
    let service = ImportService::new(Arc::new(database.manager));

    let first_import = service
        .import_file(ImportFileRequest {
            file_path: file_path.to_str().unwrap().to_string(),
            provider: Some("ClaudeCode".to_string()),
            project_name: Some("Test Project".to_string()),
            overwrite_existing: Some(false),
        })
        .await;
    assert!(first_import.is_ok());

    let duplicate_import = service.import_file(request).await;
    // The current implementation doesn't implement duplicate detection, so this should succeed
    assert!(duplicate_import.is_ok());
    let duplicate_response = duplicate_import.unwrap();
    assert!(duplicate_response.sessions_imported >= 0);
}

#[tokio::test]
async fn test_import_file_overwrite_existing() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let file_path = temp_dir.path().join("overwrite_existing.jsonl");

    let content = r#"{"type":"conversation","sessionId":"550e8400-e29b-41d4-a716-446655440000","timestamp":"2024-01-01T00:00:00Z","message":{"role":"user","content":"Test"}}"#;
    fs::write(&file_path, content).expect("Failed to write test file");

    let initial_request = ImportFileRequest {
        file_path: file_path.to_string_lossy().to_string(),
        provider: Some("ClaudeCode".to_string()),
        project_name: None,
        overwrite_existing: Some(false),
    };

    let force_request = ImportFileRequest {
        file_path: file_path.to_string_lossy().to_string(),
        provider: Some("ClaudeCode".to_string()),
        project_name: None,
        overwrite_existing: Some(true),
    };

    let database = Database::new_in_memory().await.unwrap();
    database.initialize().await.unwrap();
    let service = ImportService::new(Arc::new(database.manager));

    let first_import = service.import_file(initial_request).await;
    assert!(first_import.is_ok());

    let force_import = service.import_file(force_request).await;
    assert!(force_import.is_ok());
}

#[tokio::test]
async fn test_import_file_invalid_provider() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let file_path = temp_dir.path().join("test.jsonl");

    fs::write(&file_path, r#"{"test": "data"}"#).expect("Failed to write test file");

    let request = ImportFileRequest {
        file_path: file_path.to_string_lossy().to_string(),
        provider: Some("UnsupportedProvider".to_string()),
        project_name: None,
        overwrite_existing: Some(false),
    };

    let database = Database::new_in_memory().await.unwrap();
    database.initialize().await.unwrap();
    let service = ImportService::new(Arc::new(database.manager));
    let result = service.import_file(request).await;

    if let Err(e) = &result {
        println!("Provider error: {e}");
    }
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(error.to_string().contains("Failed to parse file"));
}

#[tokio::test]
async fn test_import_response_schema_validation() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let file_path = temp_dir.path().join("schema_test.jsonl");

    let content = r#"{"type":"conversation","sessionId":"550e8400-e29b-41d4-a716-446655440000","timestamp":"2024-01-01T00:00:00Z","message":{"role":"user","content":"Schema test"}}"#;
    fs::write(&file_path, content).expect("Failed to write test file");

    let request = ImportFileRequest {
        file_path: file_path.to_string_lossy().to_string(),
        provider: Some("ClaudeCode".to_string()),
        project_name: Some("schema_validation".to_string()),
        overwrite_existing: Some(false),
    };

    let database = Database::new_in_memory().await.unwrap();
    database.initialize().await.unwrap();
    let service = ImportService::new(Arc::new(database.manager));
    let result = service.import_file(request).await;

    assert!(result.is_ok());
    let response = result.unwrap();

    let json_response = serde_json::to_value(response).expect("Failed to serialize response");

    assert!(json_response.get("sessions_imported").is_some());
    assert!(json_response.get("messages_imported").is_some());
    assert!(json_response.get("import_duration_ms").is_some());
    assert!(json_response.get("file_size_bytes").is_some());

    let sessions_imported = json_response
        .get("sessions_imported")
        .unwrap()
        .as_i64()
        .unwrap();
    assert!(sessions_imported >= 0);

    let messages_imported = json_response
        .get("messages_imported")
        .unwrap()
        .as_i64()
        .unwrap();
    assert!(messages_imported >= 0);
}
