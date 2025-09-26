// Integration test for basic session analysis scenario
// This test MUST FAIL until full retrospection implementation is complete

use anyhow::Result;
use chrono::Utc;
// TODO: CLI analytics module not implemented yet
// use retrochat::cli::analytics::AnalyticsCommand;
use retrochat::database::chat_session_repo::ChatSessionRepository;
use retrochat::database::connection::DatabaseManager;
use retrochat::database::message_repo::MessageRepository;
use retrochat::models::chat_session::ChatSession;
use retrochat::models::message::{Message, MessageRole};
use retrochat::models::LlmProvider;
use std::env;
use uuid::Uuid;

/// Test the complete basic session analysis workflow from quickstart scenario 1
#[tokio::test]
async fn test_basic_session_analysis_workflow() -> Result<()> {
    // Skip test if no API key is available
    if env::var("GEMINI_API_KEY").is_err() {
        println!("Skipping session analysis test - GEMINI_API_KEY not set");
        return Ok(());
    }

    let db_manager = DatabaseManager::new(":memory:")?;

    // Set up test data: create a chat session
    let session_id = Uuid::new_v4();
    let session = ChatSession {
        id: session_id,
        provider: LlmProvider::ClaudeCode,
        project_name: Some("test_project".to_string()),
        start_time: Utc::now(),
        end_time: Some(Utc::now()),
        message_count: 2,
        token_count: Some(150),
        file_path: "/test/path".to_string(),
        file_hash: "test_hash".to_string(),
        created_at: Utc::now(),
        updated_at: Utc::now(),
        state: retrochat::models::chat_session::SessionState::Analyzed,
    };

    let session_repo = ChatSessionRepository::new(db_manager.clone());
    session_repo.create(&session)?;

    // Add messages to the session
    let message_repo = MessageRepository::new(db_manager.clone());
    let messages = vec![
        Message {
            id: Uuid::new_v4(),
            session_id,
            role: Role::User,
            content: "How do I implement error handling in Rust?".to_string(),
            timestamp: Utc::now(),
            token_count: Some(50),
            tool_calls: None,
            metadata: None,
            sequence_number: 0,
        },
        Message {
            id: Uuid::new_v4(),
            session_id,
            role: Role::Assistant,
            content: "In Rust, you can use Result<T, E> for error handling. Here's an example: fn divide(a: f64, b: f64) -> Result<f64, String> { if b == 0.0 { Err(\"Division by zero\".to_string()) } else { Ok(a / b) } }".to_string(),
            timestamp: Utc::now(),
            token_count: Some(100),
            tool_calls: None,
            metadata: None,
            sequence_number: 1,
        },
    ];

    for message in messages {
        message_repo.create(&message)?;
    }

    // Test CLI command: retrochat analyze retrospect --session <session-id>
    let analytics_cmd = AnalyticsCommand::new(db_manager.clone());
    let result = analytics_cmd
        .execute_retrospect_analysis(session_id, None)
        .await?;

    // Verify analysis was created
    assert!(result.is_some(), "Analysis should be created");
    let analysis = result.unwrap();

    assert_eq!(analysis.session_id, session_id);
    assert!(
        !analysis.analysis_content.is_empty(),
        "Analysis content should not be empty"
    );
    assert_eq!(
        analysis.status,
        retrochat::models::retrospection_analysis::AnalysisStatus::Complete
    );

    // Verify analysis content contains relevant insights
    let content = analysis.analysis_content.to_lowercase();
    assert!(
        content.contains("rust") || content.contains("error"),
        "Analysis should mention key topics from conversation"
    );

    // Test CLI command: retrochat analyze show --session <session-id>
    let analyses = analytics_cmd.get_session_analyses(session_id).await?;
    assert_eq!(
        analyses.len(),
        1,
        "Should have one analysis for the session"
    );
    assert_eq!(analyses[0].id, analysis.id);

    Ok(())
}

/// Test analysis with non-existent session
#[tokio::test]
async fn test_analysis_nonexistent_session() -> Result<()> {
    let db_manager = DatabaseManager::new(":memory:")?;
    let analytics_cmd = AnalyticsCommand::new(db_manager);

    let fake_session_id = Uuid::new_v4();
    let result = analytics_cmd
        .execute_retrospect_analysis(fake_session_id, None)
        .await;

    assert!(
        result.is_err(),
        "Should return error for non-existent session"
    );

    Ok(())
}

/// Test analysis output formatting
#[tokio::test]
async fn test_analysis_output_formatting() -> Result<()> {
    // Skip test if no API key is available
    if env::var("GEMINI_API_KEY").is_err() {
        println!("Skipping output formatting test - GEMINI_API_KEY not set");
        return Ok(());
    }

    let db_manager = DatabaseManager::new(":memory:")?;

    // Create test session with rich content
    let session_id = Uuid::new_v4();
    let session = ChatSession {
        id: session_id,
        provider: LlmProvider::ClaudeCode,
        project_name: Some("web_project".to_string()),
        start_time: Utc::now(),
        end_time: Some(Utc::now()),
        message_count: 4,
        token_count: Some(300),
        file_path: "/test/web_project".to_string(),
        file_hash: "web_hash".to_string(),
        created_at: Utc::now(),
        updated_at: Utc::now(),
        state: retrochat::models::chat_session::SessionState::Analyzed,
    };

    let session_repo = ChatSessionRepository::new(db_manager.clone());
    session_repo.create(&session)?;

    // Add comprehensive conversation
    let message_repo = MessageRepository::new(db_manager.clone());
    let messages = vec![
        Message {
            id: Uuid::new_v4(),
            session_id,
            role: Role::User,
            content: "I need help building a React component for user authentication".to_string(),
            timestamp: Utc::now(),
            token_count: Some(50),
            tool_calls: None,
            metadata: None,
            sequence_number: 0,
        },
        Message {
            id: Uuid::new_v4(),
            session_id,
            role: Role::Assistant,
            content: "I'll help you create a React authentication component. Let's start with a login form using React hooks...".to_string(),
            timestamp: Utc::now(),
            token_count: Some(100),
            tool_calls: None,
            metadata: None,
            sequence_number: 1,
        },
        Message {
            id: Uuid::new_v4(),
            session_id,
            role: Role::User,
            content: "How do I handle form validation and error states?".to_string(),
            timestamp: Utc::now(),
            token_count: Some(40),
            tool_calls: None,
            metadata: None,
            sequence_number: 2,
        },
        Message {
            id: Uuid::new_v4(),
            session_id,
            role: Role::Assistant,
            content: "For form validation, you can use controlled components with state management. Here's how to handle errors...".to_string(),
            timestamp: Utc::now(),
            token_count: Some(110),
            tool_calls: None,
            metadata: None,
            sequence_number: 3,
        },
    ];

    for message in messages {
        message_repo.create(&message)?;
    }

    let analytics_cmd = AnalyticsCommand::new(db_manager.clone());
    let result = analytics_cmd
        .execute_retrospect_analysis(session_id, None)
        .await?;

    let analysis = result.unwrap();

    // Verify formatted output contains structured sections
    let content = &analysis.analysis_content;

    // Should contain some structure (this depends on the prompt template)
    assert!(!content.is_empty(), "Analysis should not be empty");
    assert!(content.len() > 50, "Analysis should be substantial");

    // Test metadata formatting
    assert!(
        analysis.metadata.total_tokens > 0,
        "Should track token usage"
    );
    assert!(
        analysis.metadata.estimated_cost >= 0.0,
        "Should estimate cost"
    );
    assert!(
        analysis.metadata.execution_time_ms > 0,
        "Should track execution time"
    );

    Ok(())
}
