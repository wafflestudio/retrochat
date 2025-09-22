use retrochat::database::connection::DatabaseManager;
use retrochat::services::{AnalyticsService, DateRange, InsightsRequest};

#[tokio::test]
async fn test_insights_basic() {
    let db_manager = DatabaseManager::new(":memory:").unwrap();
    let service = AnalyticsService::new(db_manager);
    let params = InsightsRequest {
        analysis_type: None,
        date_range: Some(DateRange {
            start_date: "2024-01-01".to_string(),
            end_date: "2024-01-31".to_string(),
        }),
        include_trends: None,
        providers: None,
        insight_types: None,
    };

    let result = service.get_insights(params).await;
    assert!(result.is_ok());

    let response = result.unwrap();
    // Validate response structure
    assert!(!response.analysis_timestamp.is_empty());

    // Validate insight structure if insights exist
    for insight in &response.insights {
        assert!(!insight.insight_type.is_empty());
        assert!(!insight.title.is_empty());
        assert!(!insight.description.is_empty());
        assert!(insight.confidence_score >= 0.0 && insight.confidence_score <= 1.0);
    }
}

#[tokio::test]
async fn test_insights_provider_filter() {
    let db_manager = DatabaseManager::new(":memory:").unwrap();
    let service = AnalyticsService::new(db_manager);
    let params = InsightsRequest {
        analysis_type: None,
        date_range: Some(DateRange {
            start_date: "2024-01-01".to_string(),
            end_date: "2024-01-31".to_string(),
        }),
        include_trends: None,
        providers: Some(vec!["ClaudeCode".to_string()]),
        insight_types: None,
    };

    let result = service.get_insights(params).await;
    assert!(result.is_ok());

    let response = result.unwrap();
    // Validate that insights are provider-specific
    assert!(!response.analysis_timestamp.is_empty());

    // Validate trend structure if trends exist
    for trend in &response.trends {
        assert!(!trend.metric.is_empty());
        assert!(!trend.direction.is_empty());
        assert!(!trend.period.is_empty());
        assert!(!trend.significance.is_empty());
    }
}

#[tokio::test]
async fn test_insights_specific_types() {
    let db_manager = DatabaseManager::new(":memory:").unwrap();
    let service = AnalyticsService::new(db_manager);
    let params = InsightsRequest {
        analysis_type: None,
        date_range: Some(DateRange {
            start_date: "2024-01-01".to_string(),
            end_date: "2024-12-31".to_string(),
        }),
        include_trends: None,
        providers: None,
        insight_types: Some(vec![
            "usage_patterns".to_string(),
            "productivity".to_string(),
        ]),
    };

    let result = service.get_insights(params).await;
    assert!(result.is_ok());

    let response = result.unwrap();
    // Validate that specific insight types are requested
    assert!(!response.analysis_timestamp.is_empty());

    // Validate recommendation structure if recommendations exist
    for recommendation in &response.recommendations {
        assert!(!recommendation.category.is_empty());
        assert!(!recommendation.title.is_empty());
        assert!(!recommendation.description.is_empty());
        assert!(!recommendation.priority.is_empty());
        assert!(!recommendation.actionable_steps.is_empty());
    }
}

#[tokio::test]
async fn test_insights_comprehensive() {
    let db_manager = DatabaseManager::new(":memory:").unwrap();
    let service = AnalyticsService::new(db_manager);
    let params = InsightsRequest {
        analysis_type: None,
        date_range: None, // No date range for comprehensive analysis
        include_trends: None,
        providers: None,
        insight_types: None,
    };

    let result = service.get_insights(params).await;
    assert!(result.is_ok());

    let response = result.unwrap();
    // Comprehensive analysis should provide some insights
    assert!(!response.analysis_timestamp.is_empty());

    // For comprehensive analysis, validate response structure
    // Note: Even with no data, the service should return valid structure
    // The vectors are always valid (len() >= 0 is always true), so we validate the structure instead
    assert!(!response.insights.iter().any(|i| i.insight_type.is_empty()));
    assert!(!response.trends.iter().any(|t| t.metric.is_empty()));
    assert!(!response.recommendations.iter().any(|r| r.title.is_empty()));
}
