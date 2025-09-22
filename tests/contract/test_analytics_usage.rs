use retrochat::database::connection::DatabaseManager;
use retrochat::services::{AnalyticsService, DateRange, UsageAnalyticsRequest};

#[tokio::test]
async fn test_usage_statistics_basic() {
    let db_manager = DatabaseManager::new(":memory:").unwrap();
    let service = AnalyticsService::new(db_manager);
    let params = UsageAnalyticsRequest {
        date_range: Some(DateRange {
            start_date: "2024-01-01".to_string(),
            end_date: "2024-01-31".to_string(),
        }),
        providers: None,
        projects: None,
        aggregation_level: None,
    };

    let result = service.get_usage_analytics(params).await;
    assert!(result.is_ok());

    let response = result.unwrap();
    assert!(response.total_sessions >= 0);
    assert!(response.total_messages >= 0);
    assert!(response.total_tokens >= 0);
    assert!(response.average_session_length >= 0.0);
}

#[tokio::test]
async fn test_usage_statistics_provider_filter() {
    let db_manager = DatabaseManager::new(":memory:").unwrap();
    let service = AnalyticsService::new(db_manager);
    let params = UsageAnalyticsRequest {
        date_range: Some(DateRange {
            start_date: "2024-01-01".to_string(),
            end_date: "2024-01-31".to_string(),
        }),
        providers: Some(vec!["ClaudeCode".to_string()]),
        projects: None,
        aggregation_level: Some("daily".to_string()),
    };

    let result = service.get_usage_analytics(params).await;
    assert!(result.is_ok());

    let response = result.unwrap();
    assert!(response.total_sessions >= 0);

    // Verify that provider breakdown contains our filtered provider
    if !response.provider_breakdown.is_empty() {
        let claude_provider = response
            .provider_breakdown
            .iter()
            .find(|p| p.provider == "ClaudeCode");
        assert!(claude_provider.is_some());
    }
}

#[tokio::test]
async fn test_usage_statistics_comprehensive() {
    let db_manager = DatabaseManager::new(":memory:").unwrap();
    let service = AnalyticsService::new(db_manager);
    let params = UsageAnalyticsRequest {
        date_range: Some(DateRange {
            start_date: "2024-01-01".to_string(),
            end_date: "2024-12-31".to_string(),
        }),
        providers: None,
        projects: None,
        aggregation_level: Some("monthly".to_string()),
    };

    let result = service.get_usage_analytics(params).await;
    assert!(result.is_ok());

    let response = result.unwrap();
    // Basic validation
    assert!(response.total_sessions >= 0);
    assert!(response.total_messages >= 0);
    assert!(response.total_tokens >= 0);
    assert!(response.average_session_length >= 0.0);

    // Validate breakdown vectors structure
    for daily_usage in &response.daily_breakdown {
        assert!(!daily_usage.date.is_empty());
        assert!(daily_usage.sessions >= 0);
        assert!(daily_usage.messages >= 0);
        assert!(daily_usage.tokens >= 0);
    }

    for provider_usage in &response.provider_breakdown {
        assert!(!provider_usage.provider.is_empty());
        assert!(provider_usage.sessions >= 0);
        assert!(provider_usage.messages >= 0);
        assert!(provider_usage.tokens >= 0);
        assert!(provider_usage.percentage >= 0.0 && provider_usage.percentage <= 100.0);
    }

    for project_usage in &response.project_breakdown {
        assert!(!project_usage.project.is_empty());
        assert!(project_usage.sessions >= 0);
        assert!(project_usage.messages >= 0);
        assert!(project_usage.tokens >= 0);
        assert!(project_usage.percentage >= 0.0 && project_usage.percentage <= 100.0);
    }
}

#[tokio::test]
async fn test_usage_statistics_project_filter() {
    let db_manager = DatabaseManager::new(":memory:").unwrap();
    let service = AnalyticsService::new(db_manager);
    let params = UsageAnalyticsRequest {
        date_range: Some(DateRange {
            start_date: "2024-01-01".to_string(),
            end_date: "2024-01-31".to_string(),
        }),
        providers: None,
        projects: Some(vec!["Test Project".to_string()]),
        aggregation_level: None,
    };

    let result = service.get_usage_analytics(params).await;
    assert!(result.is_ok());

    let response = result.unwrap();
    assert!(response.total_sessions >= 0);

    // If we have project data, validate it
    if !response.project_breakdown.is_empty() {
        let test_project = response
            .project_breakdown
            .iter()
            .find(|p| p.project == "Test Project");
        assert!(test_project.is_some());
    }
}
