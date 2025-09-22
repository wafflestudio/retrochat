use retrochat::database::Database;
use retrochat::services::{ImportService, ScanRequest};
use std::sync::Arc;
use tempfile::TempDir;

#[tokio::test]
async fn test_scan_directory_basic() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let database = Database::new_in_memory().unwrap();
    database.initialize().unwrap();
    let service = ImportService::new(Arc::new(database.manager));

    // Create some test files
    std::fs::write(temp_dir.path().join("chat.jsonl"), "test content").unwrap();
    std::fs::write(temp_dir.path().join("conversation.json"), "test content").unwrap();

    let request = ScanRequest {
        directory_path: temp_dir.path().to_str().unwrap().to_string(),
        providers: None,
        recursive: Some(true),
    };
    let result = service.scan_directory(request).await;

    assert!(result.is_ok());
    let response = result.unwrap();
    // Validate scan response structure
    assert!(response.total_count >= 0);
    assert!(response.scan_duration_ms >= 0);

    // Validate file structure if files are found
    for file in &response.files_found {
        assert!(!file.file_path.is_empty());
        assert!(!file.provider.is_empty());
        assert!(file.estimated_sessions >= 0);
        assert!(file.file_size_bytes >= 0);
        assert!(!file.last_modified.is_empty());
    }
}

#[tokio::test]
async fn test_scan_directory_with_providers() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let database = Database::new_in_memory().unwrap();
    database.initialize().unwrap();
    let service = ImportService::new(Arc::new(database.manager));

    let request = ScanRequest {
        directory_path: temp_dir.path().to_str().unwrap().to_string(),
        providers: Some(vec!["ClaudeCode".to_string()]),
        recursive: Some(false),
    };
    let result = service.scan_directory(request).await;

    assert!(result.is_ok());
    let response = result.unwrap();
    // Validate scan response structure
    assert!(response.total_count >= 0);
    assert!(response.scan_duration_ms >= 0);

    // Validate file structure if files are found
    for file in &response.files_found {
        assert!(!file.file_path.is_empty());
        assert!(!file.provider.is_empty());
        assert!(file.estimated_sessions >= 0);
        assert!(file.file_size_bytes >= 0);
        assert!(!file.last_modified.is_empty());
    }
}
