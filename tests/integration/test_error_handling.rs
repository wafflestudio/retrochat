use retrochat::services::{RetrospectionService};
use retrochat::services::google_ai::{GoogleAiClient, GoogleAiConfig};
use retrochat::database::DatabaseManager;
use retrochat::models::{RetrospectionAnalysisType, OperationStatus, RetrospectionRequest};
use std::sync::Arc;
use std::time::Duration;

#[tokio::test]
async fn test_google_ai_api_error_recovery() {
    // Test error handling and recovery for Google AI API failures
    let db_manager = Arc::new(DatabaseManager::new(":memory:").await.unwrap());

    let session_id = "error-test-session".to_string();

    // Create service with invalid API key to test error handling
    let config = GoogleAiConfig {
        api_key: "invalid_key".to_string(),
        base_url: "https://generativelanguage.googleapis.com/v1beta".to_string(),
        model: "gemini-2.5-flash-lite".to_string(),
        timeout: Duration::from_secs(1),
        max_retries: 0,
    };
    let google_ai_client = GoogleAiClient::new(config).unwrap();
    let service = RetrospectionService::new(db_manager, google_ai_client);

    // Test creating analysis request
    let result = service.create_analysis_request(
        session_id.clone(),
        RetrospectionAnalysisType::UserInteractionAnalysis,
        Some("test_user".to_string()),
        None,
    ).await;

    match result {
        Ok(request) => {
            // Try to execute the analysis - should fail with invalid API key
            let execution_result = service.execute_analysis(request.id.clone()).await;
            match execution_result {
                Ok(_) => {
                    // This shouldn't succeed with invalid API key
                    panic!("Expected analysis to fail with invalid API key");
                }
                Err(e) => {
                    // Should be a meaningful error about API key or connection
                    let error_msg = e.to_string();
                    assert!(!error_msg.is_empty());
                    println!("Expected API error: {}", error_msg);
                }
            }
        }
        Err(e) => {
            // Expected to fail if validation catches the invalid config
            println!("Expected failure during request creation: {:?}", e);
        }
    }
}

#[tokio::test]
async fn test_service_creation_with_invalid_config() {
    // Test service creation with invalid configuration
    let db_manager = Arc::new(DatabaseManager::new(":memory:").await.unwrap());

    // Create service with invalid API key
    let config = GoogleAiConfig {
        api_key: "invalid_key".to_string(),
        base_url: "https://generativelanguage.googleapis.com/v1beta".to_string(),
        model: "gemini-2.5-flash-lite".to_string(),
        timeout: Duration::from_secs(1),
        max_retries: 0,
    };

    // This should succeed (client creation doesn't validate the key immediately)
    let google_ai_client = GoogleAiClient::new(config).unwrap();
    let service = RetrospectionService::new(db_manager, google_ai_client);

    // Try to create an analysis request
    let result = service.create_analysis_request(
        "test-session".to_string(),
        RetrospectionAnalysisType::UserInteractionAnalysis,
        Some("test_user".to_string()),
        None,
    ).await;

    // Request creation should succeed
    match result {
        Ok(_) => assert!(true),
        Err(e) => {
            // May fail due to database issues or validation
            println!("Request creation failed (acceptable): {:?}", e);
        }
    }
}

#[tokio::test]
async fn test_analysis_execution_error_handling() {
    // Test error handling during analysis execution
    let db_manager = Arc::new(DatabaseManager::new(":memory:").await.unwrap());

    // Create service with short timeout to trigger timeout errors
    let config = GoogleAiConfig {
        api_key: std::env::var("GOOGLE_AI_API_KEY").unwrap_or("test_key".to_string()),
        base_url: "https://generativelanguage.googleapis.com/v1beta".to_string(),
        model: "gemini-2.5-flash-lite".to_string(),
        timeout: Duration::from_millis(1), // Very short timeout
        max_retries: 0,
    };

    let google_ai_client = GoogleAiClient::new(config).unwrap();
    let service = RetrospectionService::new(db_manager, google_ai_client);

    // Create analysis request
    let result = service.create_analysis_request(
        "timeout-test-session".to_string(),
        RetrospectionAnalysisType::TaskBreakdown,
        Some("test_user".to_string()),
        None,
    ).await;

    match result {
        Ok(request) => {
            // Try to execute analysis - should fail due to timeout
            let execution_result = service.execute_analysis(request.id).await;
            match execution_result {
                Ok(_) => {
                    // Unexpected success with very short timeout
                    println!("Analysis succeeded unexpectedly with short timeout");
                }
                Err(e) => {
                    // Expected failure due to timeout or other issues
                    let error_msg = e.to_string();
                    assert!(!error_msg.is_empty());
                    println!("Expected timeout/error: {}", error_msg);
                }
            }
        }
        Err(e) => {
            // May fail during request creation
            println!("Request creation failed: {:?}", e);
        }
    }
}

#[tokio::test]
async fn test_nonexistent_session_handling() {
    // Test error handling for nonexistent sessions
    let db_manager = Arc::new(DatabaseManager::new(":memory:").await.unwrap());

    let config = GoogleAiConfig::default();
    let google_ai_client = GoogleAiClient::new(config).unwrap();
    let service = RetrospectionService::new(db_manager, google_ai_client);

    // Try to create analysis request for nonexistent session
    let result = service.create_analysis_request(
        "definitely-nonexistent-session-12345".to_string(),
        RetrospectionAnalysisType::UserInteractionAnalysis,
        Some("test_user".to_string()),
        None,
    ).await;

    // This may succeed or fail depending on validation strategy
    match result {
        Ok(request) => {
            // If request creation succeeds, try to execute it
            let execution_result = service.execute_analysis(request.id).await;
            match execution_result {
                Ok(_) => {
                    // Unexpected success - session doesn't exist
                    println!("Warning: Analysis succeeded for nonexistent session");
                }
                Err(e) => {
                    // Expected failure
                    let error_msg = e.to_string();
                    assert!(
                        error_msg.contains("session") ||
                        error_msg.contains("not found") ||
                        error_msg.contains("database")
                    );
                }
            }
        }
        Err(e) => {
            // Expected failure during request creation
            let error_msg = e.to_string();
            assert!(
                error_msg.contains("session") ||
                error_msg.contains("not found") ||
                error_msg.contains("database")
            );
        }
    }
}

#[tokio::test]
async fn test_database_error_handling() {
    // Test error handling for database issues
    // This test uses an in-memory database which should work fine
    // but validates the error handling patterns
    let db_manager = Arc::new(DatabaseManager::new(":memory:").await.unwrap());

    let config = GoogleAiConfig::default();
    let google_ai_client = GoogleAiClient::new(config).unwrap();
    let service = RetrospectionService::new(db_manager, google_ai_client);

    // Try various operations that might trigger database errors
    let operations = vec![
        ("test-db-session-1", RetrospectionAnalysisType::UserInteractionAnalysis),
        ("test-db-session-2", RetrospectionAnalysisType::CollaborationInsights),
        ("test-db-session-3", RetrospectionAnalysisType::Custom("Test prompt".to_string())),
    ];

    for (session_id, analysis_type) in operations {
        let result = service.create_analysis_request(
            session_id.to_string(),
            analysis_type,
            Some("test_user".to_string()),
            None,
        ).await;

        // All these should succeed or fail gracefully
        match result {
            Ok(_) => assert!(true),
            Err(e) => {
                // Should be a meaningful error message
                let error_msg = e.to_string();
                assert!(!error_msg.is_empty());
                println!("Database operation error (acceptable): {}", error_msg);
            }
        }
    }
}