use retrochat::database::connection::DatabaseManager;
use retrochat::services::query_service::{
    DateRange, QueryService, SessionFilters, SessionsQueryRequest,
};
use std::sync::Arc;

#[tokio::test]
async fn test_list_sessions_default() {
    let service = QueryService::new().await;
    let request = SessionsQueryRequest {
        page: None,
        page_size: None,
        sort_by: None,
        sort_order: None,
        filters: None,
    };

    let result = service.query_sessions(request).await;
    assert!(result.is_ok());

    let response = result.unwrap();
    assert!(response.sessions.len() <= 50); // Default limit
    assert!(response.total_count >= 0);
}

#[tokio::test]
async fn test_list_sessions_filter_by_provider() {
    let service = QueryService::new().await;
    let request = SessionsQueryRequest {
        page: None,
        page_size: None,
        sort_by: None,
        sort_order: None,
        filters: Some(SessionFilters {
            provider: Some("ClaudeCode".to_string()),
            project: None,
            date_range: None,
            min_messages: None,
            max_messages: None,
        }),
    };

    let result = service.query_sessions(request).await;
    assert!(result.is_ok());

    let response = result.unwrap();
    assert!(response.sessions.iter().all(|s| s.provider == "ClaudeCode"));
}

#[tokio::test]
async fn test_list_sessions_filter_by_project() {
    let service = QueryService::new().await;
    let request = SessionsQueryRequest {
        page: None,
        page_size: None,
        sort_by: None,
        sort_order: None,
        filters: Some(SessionFilters {
            provider: None,
            project: Some("test-project".to_string()),
            date_range: None,
            min_messages: None,
            max_messages: None,
        }),
    };

    let result = service.query_sessions(request).await;
    assert!(result.is_ok());

    let response = result.unwrap();
    assert!(response
        .sessions
        .iter()
        .all(|s| s.project.as_deref() == Some("test-project")));
}

#[tokio::test]
async fn test_list_sessions_date_range() {
    // Use a test database instead of the main database
    let db_manager = Arc::new(DatabaseManager::open_in_memory().await.unwrap());
    let service = QueryService::with_database(db_manager);

    let request = SessionsQueryRequest {
        page: None,
        page_size: None,
        sort_by: None,
        sort_order: None,
        filters: Some(SessionFilters {
            provider: None,
            project: None,
            date_range: Some(DateRange {
                start_date: "2024-01-01".to_string(),
                end_date: "2024-12-31".to_string(),
            }),
            min_messages: None,
            max_messages: None,
        }),
    };

    let result = service.query_sessions(request).await;
    assert!(result.is_ok());

    let response = result.unwrap();
    // With an empty test database, there should be no sessions
    assert_eq!(response.sessions.len(), 0);
    assert_eq!(response.total_count, 0);
}

#[tokio::test]
async fn test_list_sessions_pagination() {
    let service = QueryService::new().await;

    let first_page = SessionsQueryRequest {
        page: Some(1),
        page_size: Some(10),
        sort_by: None,
        sort_order: None,
        filters: None,
    };

    let second_page = SessionsQueryRequest {
        page: Some(2),
        page_size: Some(10),
        sort_by: None,
        sort_order: None,
        filters: None,
    };

    let first_result = service.query_sessions(first_page).await;
    let second_result = service.query_sessions(second_page).await;

    assert!(first_result.is_ok());
    assert!(second_result.is_ok());

    let first_response = first_result.unwrap();
    let second_response = second_result.unwrap();

    assert!(first_response.sessions.len() <= 10);
    assert!(second_response.sessions.len() <= 10);
    assert_eq!(first_response.total_count, second_response.total_count);
}

#[tokio::test]
async fn test_list_sessions_sorting() {
    let service = QueryService::new().await;

    let request_asc = SessionsQueryRequest {
        page: None,
        page_size: Some(20),
        sort_by: Some("start_time".to_string()),
        sort_order: Some("asc".to_string()),
        filters: None,
    };

    let request_desc = SessionsQueryRequest {
        page: None,
        page_size: Some(20),
        sort_by: Some("start_time".to_string()),
        sort_order: Some("desc".to_string()),
        filters: None,
    };

    let asc_result = service.query_sessions(request_asc).await;
    let desc_result = service.query_sessions(request_desc).await;

    assert!(asc_result.is_ok());
    assert!(desc_result.is_ok());

    let _asc_response = asc_result.unwrap();
    let _desc_response = desc_result.unwrap();
}

#[tokio::test]
async fn test_sessions_response_schema() {
    let service = QueryService::new().await;
    let request = SessionsQueryRequest {
        page: None,
        page_size: Some(5),
        sort_by: None,
        sort_order: None,
        filters: None,
    };

    let result = service.query_sessions(request).await;
    assert!(result.is_ok());

    let response = result.unwrap();
    let json_response = serde_json::to_value(response).expect("Failed to serialize response");

    assert!(json_response.get("sessions").is_some());
    assert!(json_response.get("total_count").is_some());
    assert!(json_response.get("page").is_some());
    assert!(json_response.get("page_size").is_some());
    assert!(json_response.get("total_pages").is_some());

    if let Some(sessions) = json_response.get("sessions").and_then(|s| s.as_array()) {
        for session in sessions {
            assert!(session.get("session_id").is_some());
            assert!(session.get("provider").is_some());
            assert!(session.get("start_time").is_some());
            assert!(session.get("message_count").is_some());
        }
    }
}
