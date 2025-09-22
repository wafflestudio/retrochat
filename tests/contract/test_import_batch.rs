use retrochat::database::Database;
use retrochat::services::{BatchImportRequest, ImportService};
use std::sync::Arc;
use tempfile::TempDir;

#[tokio::test]
async fn test_batch_import_basic() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let database = Database::new_in_memory().unwrap();
    database.initialize().unwrap();
    let service = ImportService::new(Arc::new(database.manager));

    // Create some test files
    std::fs::write(temp_dir.path().join("chat.jsonl"), "test content").unwrap();

    let request = BatchImportRequest {
        directory_path: temp_dir.path().to_str().unwrap().to_string(),
        providers: None,
        project_name: Some("Test Project".to_string()),
        overwrite_existing: Some(false),
        recursive: Some(true),
    };

    let result = service.import_batch(request).await;
    assert!(result.is_ok());

    let response = result.unwrap();
    assert!(response.total_files_processed >= 0);
    assert!(response.successful_imports >= 0);
    assert!(response.failed_imports >= 0);
}

#[tokio::test]
async fn test_batch_import_with_filters() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let database = Database::new_in_memory().unwrap();
    database.initialize().unwrap();
    let service = ImportService::new(Arc::new(database.manager));

    let request = BatchImportRequest {
        directory_path: temp_dir.path().to_str().unwrap().to_string(),
        providers: Some(vec!["ClaudeCode".to_string()]),
        project_name: Some("Filtered Project".to_string()),
        overwrite_existing: Some(true),
        recursive: Some(false),
    };

    let result = service.import_batch(request).await;
    assert!(result.is_ok());
}
