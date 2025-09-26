// Contract test for retrospection service analyze functionality
// This test MUST FAIL until RetrospectionService is implemented

use anyhow::Result;
use retrochat::database::connection::DatabaseManager;
use retrochat::models::analysis_metadata::AnalysisMetadata;
use retrochat::models::analysis_request::AnalysisRequest;
use retrochat::models::retrospection_analysis::RetrospectionAnalysis;
use retrochat::services::retrospection_service::RetrospectionService;
use std::collections::HashMap;
use uuid::Uuid;

/// Test that retrospection service correctly processes analysis requests
#[tokio::test]
async fn test_retrospection_service_analyze() -> Result<()> {
    let db_manager = DatabaseManager::new(":memory:")?;
    let service = RetrospectionService::new(db_manager)?;

    // Create test analysis request
    let session_id = Uuid::new_v4();
    let template_id = "session_summary".to_string();
    let mut variables = HashMap::new();
    variables.insert(
        "chat_content".to_string(),
        "Test chat session content".to_string(),
    );

    let request = AnalysisRequest {
        id: Uuid::new_v4(),
        session_id,
        prompt_template_id: template_id.clone(),
        template_variables: variables,
        status: retrochat::models::analysis_request::RequestStatus::Queued,
        error_message: None,
        created_at: chrono::Utc::now(),
        started_at: None,
        completed_at: None,
    };

    // This should fail until service is implemented
    let result = service.process_analysis_request(request).await?;

    // Verify result structure
    assert_eq!(result.session_id, session_id);
    assert_eq!(result.prompt_template_id, template_id);
    assert!(!result.analysis_content.is_empty());
    assert_eq!(
        result.status,
        retrochat::models::retrospection_analysis::AnalysisStatus::Complete
    );

    // Verify metadata is populated
    assert!(result.metadata.prompt_tokens > 0);
    assert!(result.metadata.completion_tokens > 0);
    assert!(result.metadata.total_tokens > 0);
    assert!(result.metadata.estimated_cost >= 0.0);
    assert!(result.metadata.execution_time_ms > 0);

    Ok(())
}

/// Test error handling for invalid template ID
#[tokio::test]
async fn test_retrospection_service_invalid_template() -> Result<()> {
    let db_manager = DatabaseManager::new(":memory:")?;
    let service = RetrospectionService::new(db_manager)?;

    let request = AnalysisRequest {
        id: Uuid::new_v4(),
        session_id: Uuid::new_v4(),
        prompt_template_id: "nonexistent_template".to_string(),
        template_variables: HashMap::new(),
        status: retrochat::models::analysis_request::RequestStatus::Queued,
        error_message: None,
        created_at: chrono::Utc::now(),
        started_at: None,
        completed_at: None,
    };

    let result = service.process_analysis_request(request).await;
    assert!(result.is_err(), "Should return error for invalid template");

    Ok(())
}

/// Test analysis storage and retrieval
#[tokio::test]
async fn test_retrospection_service_storage() -> Result<()> {
    let db_manager = DatabaseManager::new(":memory:")?;
    let service = RetrospectionService::new(db_manager.clone());

    let session_id = Uuid::new_v4();
    let analysis = RetrospectionAnalysis {
        id: Uuid::new_v4(),
        session_id,
        prompt_template_id: "test_template".to_string(),
        analysis_content: "Test analysis content".to_string(),
        metadata: AnalysisMetadata {
            llm_service: "gemini-2.5-flash-lite".to_string(),
            prompt_tokens: 100,
            completion_tokens: 50,
            total_tokens: 150,
            estimated_cost: 0.001,
            execution_time_ms: 1500,
            api_response_metadata: None,
        },
        status: retrochat::models::retrospection_analysis::AnalysisStatus::Complete,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };

    // Store analysis
    service.store_analysis(&analysis).await?;

    // Retrieve analyses for session
    let retrieved = service.get_analyses_for_session(session_id).await?;
    assert_eq!(retrieved.len(), 1);
    assert_eq!(retrieved[0].id, analysis.id);
    assert_eq!(retrieved[0].analysis_content, analysis.analysis_content);

    Ok(())
}

/// Test concurrent analysis processing
#[tokio::test]
async fn test_retrospection_service_concurrent_processing() -> Result<()> {
    let db_manager = DatabaseManager::new(":memory:")?;
    let service = RetrospectionService::new(db_manager)?;

    let mut handles = Vec::new();

    // Launch multiple analysis requests concurrently
    for i in 0..3 {
        let service_clone = service.clone();
        let handle = tokio::spawn(async move {
            let request = AnalysisRequest {
                id: Uuid::new_v4(),
                session_id: Uuid::new_v4(),
                prompt_template_id: "session_summary".to_string(),
                template_variables: {
                    let mut vars = HashMap::new();
                    vars.insert("chat_content".to_string(), format!("Test content {}", i));
                    vars
                },
                status: retrochat::models::analysis_request::RequestStatus::Queued,
                error_message: None,
                created_at: chrono::Utc::now(),
                started_at: None,
                completed_at: None,
            };

            service_clone.process_analysis_request(request).await
        });
        handles.push(handle);
    }

    // Wait for all to complete
    for handle in handles {
        let result = handle.await??;
        assert_eq!(
            result.status,
            retrochat::models::retrospection_analysis::AnalysisStatus::Complete
        );
    }

    Ok(())
}
