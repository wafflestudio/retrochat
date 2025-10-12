use chrono::Utc;
use retrochat::database::{
    ChatSessionRepository, DatabaseManager, MessageRepository, ProjectRepository,
};
use retrochat::models::{
    {ChatSession, LlmProvider},
    message::{Message, MessageRole},
    project::Project,
};
use retrochat::services::query_service::{DateRange, QueryService, SearchRequest};
use std::sync::Arc;
use uuid::Uuid;

async fn setup_test_data() -> QueryService {
    let db_manager = DatabaseManager::new(":memory:").await.unwrap();
    let service = QueryService::with_database(Arc::new(db_manager.clone()));

    // Create test projects first (required by foreign key constraint)
    let project_repo = ProjectRepository::new(&db_manager);
    let project1 = Project {
        id: Uuid::parse_str("550e8400-e29b-41d4-a716-446655440001").unwrap(),
        name: "test-project".to_string(),
        description: Some("Test project for search".to_string()),
        working_directory: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        session_count: 0,
        total_tokens: 0,
    };
    let project2 = Project {
        id: Uuid::parse_str("550e8400-e29b-41d4-a716-446655440002").unwrap(),
        name: "another-project".to_string(),
        description: Some("Another test project for search".to_string()),
        working_directory: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        session_count: 0,
        total_tokens: 0,
    };
    project_repo.create(&project1).await.unwrap();
    project_repo.create(&project2).await.unwrap();

    // Create test sessions
    let session_repo = ChatSessionRepository::new(&db_manager);
    let message_repo = MessageRepository::new(&db_manager);

    // Session 1: ClaudeCode with test-project
    let mut session1 = ChatSession::new(
        LlmProvider::ClaudeCode,
        "test.jsonl".to_string(),
        "hash123".to_string(),
        Utc::now(),
    );
    session1.project_name = Some("test-project".to_string());
    session_repo.create(&session1).await.unwrap();

    // Session 2: Gemini with another-project
    let mut session2 = ChatSession::new(
        LlmProvider::GeminiCLI,
        "test2.jsonl".to_string(),
        "hash456".to_string(),
        Utc::now(),
    );
    session2.project_name = Some("another-project".to_string());
    session_repo.create(&session2).await.unwrap();

    // Create test messages
    let message1 = Message::new(
        session1.id,
        MessageRole::User,
        "Hello, this is a test message about machine learning".to_string(),
        Utc::now(),
        1,
    );
    message_repo.create(&message1).await.unwrap();

    let message2 = Message::new(
        session1.id,
        MessageRole::Assistant,
        "Hello! I can help you with machine learning concepts".to_string(),
        Utc::now(),
        2,
    );
    message_repo.create(&message2).await.unwrap();

    let message3 = Message::new(
        session2.id,
        MessageRole::User,
        "This is about code optimization and performance".to_string(),
        Utc::now(),
        1,
    );
    message_repo.create(&message3).await.unwrap();

    let message4 = Message::new(
        session2.id,
        MessageRole::Assistant,
        "I can help you optimize your code for better performance".to_string(),
        Utc::now(),
        2,
    );
    message_repo.create(&message4).await.unwrap();

    service
}

#[tokio::test]
async fn test_search_messages_basic() {
    let service = setup_test_data().await;
    let request = SearchRequest {
        query: "hello".to_string(),
        providers: None,
        projects: None,
        date_range: None,
        search_type: None,
        page: None,
        page_size: None,
    };

    let result = service.search_messages(request).await;
    assert!(result.is_ok());

    let response = result.unwrap();
    assert!(response.search_duration_ms >= 0);
    assert!(response.results.len() <= 20); // Default limit
    assert!(response.total_count >= response.results.len() as i32);

    for search_result in &response.results {
        assert!(search_result
            .content_snippet
            .to_lowercase()
            .contains("hello"));
        assert!(search_result.relevance_score >= 0.0 && search_result.relevance_score <= 1.0);
    }
}

#[tokio::test]
async fn test_search_messages_with_provider_filter() {
    let service = setup_test_data().await;
    let request = SearchRequest {
        query: "test".to_string(),
        providers: Some(vec!["ClaudeCode".to_string()]),
        projects: None,
        date_range: None,
        search_type: None,
        page: None,
        page_size: None,
    };

    let result = service.search_messages(request).await;
    assert!(result.is_ok());

    let response = result.unwrap();
    for search_result in &response.results {
        assert_eq!(search_result.provider, "claude-code");
    }
}

#[tokio::test]
async fn test_search_messages_with_project_filter() {
    let service = setup_test_data().await;
    let request = SearchRequest {
        query: "machine learning".to_string(),
        providers: None,
        projects: Some(vec!["test-project".to_string()]),
        date_range: None,
        search_type: None,
        page: None,
        page_size: None,
    };

    let result = service.search_messages(request).await;
    assert!(result.is_ok());

    let response = result.unwrap();
    for search_result in &response.results {
        assert_eq!(search_result.project.as_deref(), Some("test-project"));
    }
}

#[tokio::test]
async fn test_search_messages_with_limit() {
    let service = setup_test_data().await;
    let request = SearchRequest {
        query: "the".to_string(), // Common word likely to have many matches
        providers: None,
        projects: None,
        date_range: None,
        search_type: None,
        page: None,
        page_size: Some(5),
    };

    let result = service.search_messages(request).await;
    assert!(result.is_ok());

    let response = result.unwrap();
    assert!(response.results.len() <= 5);
}

#[tokio::test]
async fn test_search_messages_with_date_range() {
    let service = setup_test_data().await;
    let request = SearchRequest {
        query: "hello".to_string(),
        providers: None,
        projects: None,
        date_range: Some(DateRange {
            start_date: "2025-01-01".to_string(),
            end_date: "2025-12-31".to_string(),
        }),
        search_type: None,
        page: None,
        page_size: None,
    };

    let result = service.search_messages(request).await;
    assert!(result.is_ok());

    let response = result.unwrap();
    for search_result in &response.results {
        let timestamp_date = &search_result.timestamp[..10]; // Extract date part
        assert!(timestamp_date >= "2025-01-01");
        assert!(timestamp_date <= "2025-12-31");
    }
}

#[tokio::test]
async fn test_search_messages_pagination() {
    let service = setup_test_data().await;

    let first_page = SearchRequest {
        query: "test".to_string(),
        providers: None,
        projects: None,
        date_range: None,
        search_type: None,
        page: Some(1),
        page_size: Some(10),
    };

    let second_page = SearchRequest {
        query: "test".to_string(),
        providers: None,
        projects: None,
        date_range: None,
        search_type: None,
        page: Some(2),
        page_size: Some(10),
    };

    let first_result = service.search_messages(first_page).await;
    let second_result = service.search_messages(second_page).await;

    assert!(first_result.is_ok());
    assert!(second_result.is_ok());

    let first_response = first_result.unwrap();
    let second_response = second_result.unwrap();

    assert!(first_response.results.len() <= 10);
    assert!(second_response.results.len() <= 10);
    assert_eq!(first_response.total_count, second_response.total_count);
}

#[tokio::test]
async fn test_search_messages_relevance_scoring() {
    let service = setup_test_data().await;
    let request = SearchRequest {
        query: "important".to_string(),
        providers: None,
        projects: None,
        date_range: None,
        search_type: None,
        page: None,
        page_size: Some(10),
    };

    let result = service.search_messages(request).await;
    assert!(result.is_ok());

    let response = result.unwrap();
    if response.results.len() > 1 {
        // Results should be ordered by relevance score (descending)
        for i in 1..response.results.len() {
            assert!(response.results[i - 1].relevance_score >= response.results[i].relevance_score);
        }
    }

    for search_result in &response.results {
        assert!(search_result.relevance_score >= 0.0);
        assert!(search_result.relevance_score <= 1.0);
    }
}

#[tokio::test]
async fn test_search_response_schema_validation() {
    let service = setup_test_data().await;
    let request = SearchRequest {
        query: "schema test".to_string(),
        providers: None,
        projects: None,
        date_range: None,
        search_type: None,
        page: None,
        page_size: Some(3),
    };

    let result = service.search_messages(request).await;
    assert!(result.is_ok());

    let response = result.unwrap();
    let json_response = serde_json::to_value(response).expect("Failed to serialize response");

    assert!(json_response.get("results").is_some());
    assert!(json_response.get("total_count").is_some());
    assert!(json_response.get("page").is_some());
    assert!(json_response.get("page_size").is_some());
    assert!(json_response.get("search_duration_ms").is_some());

    if let Some(results) = json_response.get("results").and_then(|r| r.as_array()) {
        for result in results {
            // Validate required fields
            assert!(result.get("session_id").is_some());
            assert!(result.get("message_id").is_some());
            assert!(result.get("provider").is_some());
            assert!(result.get("timestamp").is_some());
            assert!(result.get("content_snippet").is_some());
            assert!(result.get("message_role").is_some());
            assert!(result.get("relevance_score").is_some());

            // Validate message_role enum
            let role = result.get("message_role").unwrap().as_str().unwrap();
            assert!(["user", "assistant", "system"].contains(&role));

            // Validate relevance score
            let score = result.get("relevance_score").unwrap().as_f64().unwrap();
            assert!((0.0..=1.0).contains(&score));
        }
    }
}

#[tokio::test]
async fn test_search_performance() {
    let service = setup_test_data().await;
    let request = SearchRequest {
        query: "performance test".to_string(),
        providers: None,
        projects: None,
        date_range: None,
        search_type: None,
        page: None,
        page_size: Some(20),
    };

    let start_time = std::time::Instant::now();
    let result = service.search_messages(request).await;
    let duration = start_time.elapsed();

    assert!(result.is_ok());
    let response = result.unwrap();

    // Search should complete within reasonable time
    assert!(duration.as_millis() < 5000); // 5 seconds max
    assert!(response.search_duration_ms < 5000);
}
