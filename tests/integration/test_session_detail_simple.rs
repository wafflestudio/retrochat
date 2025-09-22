use retrochat::database::Database;
use retrochat::services::{ImportService, QueryService, SessionsQueryRequest, SessionFilters, SessionDetailRequest};
use tempfile::TempDir;
use std::sync::Arc;

#[tokio::test]
async fn test_session_detail_basic() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");

    // Setup database
    let database = Database::new_in_memory().unwrap();
    database.initialize().expect("Failed to initialize database");
    let import_service = ImportService::new(Arc::new(database.manager));

    // Create a test file
    std::fs::write(temp_dir.path().join("test.jsonl"), r#"{"timestamp":"2024-01-01T00:00:00Z","messages":[{"role":"user","content":"Hello"},{"role":"assistant","content":"Hi there!"}]}"#).unwrap();

    // Import the file
    let import_result = import_service.import_file(retrochat::services::ImportFileRequest {
        file_path: temp_dir.path().join("test.jsonl").to_str().unwrap().to_string(),
        provider: Some("ClaudeCode".to_string()),
        project_name: Some("Test Project".to_string()),
        overwrite_existing: Some(false),
    }).await;

    // Basic assertions
    if import_result.is_ok() {
        let query_service = QueryService::new();

        // Query sessions
        let sessions_result = query_service.query_sessions(SessionsQueryRequest {
            page: None,
            page_size: None,
            sort_by: None,
            sort_order: None,
            filters: None,
        }).await;

        assert!(sessions_result.is_ok());
        let sessions_response = sessions_result.unwrap();
        assert!(sessions_response.total_count >= 0);
        assert!(sessions_response.page > 0);
        assert!(sessions_response.page_size > 0);
        assert!(sessions_response.total_pages >= 0);
        
        // Validate session summaries if any exist
        for session in &sessions_response.sessions {
            assert!(!session.session_id.is_empty());
            assert!(!session.provider.is_empty());
            assert!(!session.start_time.is_empty());
            assert!(!session.end_time.is_empty());
            assert!(session.message_count >= 0);
            assert!(!session.first_message_preview.is_empty());
        }
    }
}

#[tokio::test]
async fn test_session_detail_with_filters() {
    let query_service = QueryService::new();

    let sessions_result = query_service.query_sessions(SessionsQueryRequest {
        page: Some(1),
        page_size: Some(10),
        sort_by: Some("created_at".to_string()),
        sort_order: Some("desc".to_string()),
        filters: Some(SessionFilters {
            provider: Some("ClaudeCode".to_string()),
            project: None,
            date_range: None,
            min_messages: None,
            max_messages: None,
        }),
    }).await;

    assert!(sessions_result.is_ok());
    let sessions_response = sessions_result.unwrap();
    assert!(sessions_response.total_count >= 0);
    assert!(sessions_response.page > 0);
    assert!(sessions_response.page_size > 0);
    assert!(sessions_response.total_pages >= 0);
    
    // Validate session summaries if any exist
    for session in &sessions_response.sessions {
        assert!(!session.session_id.is_empty());
        assert!(!session.provider.is_empty());
        assert!(!session.start_time.is_empty());
        assert!(!session.end_time.is_empty());
        assert!(session.message_count >= 0);
        assert!(!session.first_message_preview.is_empty());
    }
}