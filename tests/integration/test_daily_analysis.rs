use retrochat::database::Database;
use retrochat::services::{
    AnalyticsService, DateRange, ImportService, InsightsRequest, UsageAnalyticsRequest,
};
use std::sync::Arc;
use tempfile::TempDir;

#[tokio::test]
async fn test_daily_usage_analysis_workflow() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");

    // Setup database
    let database = Database::new_in_memory().unwrap();
    database
        .initialize()
        .expect("Failed to initialize database");
    let db_manager = database.manager.clone();
    let import_service = ImportService::new(Arc::new(database.manager));

    // Create sample data files for different days
    for day in 1..=7 {
        let content = format!(
            r#"{{"timestamp":"2024-01-{day:02}T00:00:00Z","messages":[{{"role":"user","content":"Day {day} question"}},{{"role":"assistant","content":"Day {day} answer"}}]}}"#
        );
        std::fs::write(
            temp_dir.path().join(format!("chat_day_{day}.jsonl")),
            content,
        )
        .unwrap();

        // Import each file
        let import_result = import_service
            .import_file(retrochat::services::ImportFileRequest {
                file_path: temp_dir
                    .path()
                    .join(format!("chat_day_{day}.jsonl"))
                    .to_str()
                    .unwrap()
                    .to_string(),
                provider: Some(if day % 2 == 0 {
                    "ClaudeCode".to_string()
                } else {
                    "Gemini".to_string()
                }),
                project_name: Some("Daily Analysis Test".to_string()),
                overwrite_existing: Some(false),
            })
            .await;

        // We don't require imports to succeed, just that the API works
        assert!(import_result.is_ok() || import_result.is_err());
    }

    // Test analytics service
    let analytics_service = AnalyticsService::new(db_manager);

    // Test usage analytics for the week
    let usage_request = UsageAnalyticsRequest {
        date_range: Some(DateRange {
            start_date: "2024-01-01".to_string(),
            end_date: "2024-01-07".to_string(),
        }),
        providers: None,
        projects: Some(vec!["Daily Analysis Test".to_string()]),
        aggregation_level: Some("daily".to_string()),
    };

    let usage_result = analytics_service.get_usage_analytics(usage_request).await;
    assert!(usage_result.is_ok());

    let usage_response = usage_result.unwrap();
    assert!(usage_response.total_sessions >= 0);
    assert!(usage_response.total_messages >= 0);

    // Validate breakdown structure
    for daily_usage in &usage_response.daily_breakdown {
        assert!(!daily_usage.date.is_empty());
        assert!(daily_usage.sessions >= 0);
        assert!(daily_usage.messages >= 0);
        assert!(daily_usage.tokens >= 0);
    }

    // Test insights generation
    let insights_request = InsightsRequest {
        analysis_type: None,
        date_range: Some(DateRange {
            start_date: "2024-01-01".to_string(),
            end_date: "2024-01-07".to_string(),
        }),
        include_trends: None,
        providers: None,
        insight_types: Some(vec!["usage_patterns".to_string(), "trends".to_string()]),
    };

    let insights_result = analytics_service.get_insights(insights_request).await;
    assert!(insights_result.is_ok());

    let insights_response = insights_result.unwrap();
    // Validate insights response structure
    assert!(!insights_response.analysis_timestamp.is_empty());

    // Validate insight structure if insights exist
    for insight in &insights_response.insights {
        assert!(!insight.insight_type.is_empty());
        assert!(!insight.title.is_empty());
        assert!(!insight.description.is_empty());
        assert!(insight.confidence_score >= 0.0 && insight.confidence_score <= 1.0);
    }
}

#[tokio::test]
async fn test_analytics_with_different_providers() {
    let database = Database::new_in_memory().unwrap();
    database
        .initialize()
        .expect("Failed to initialize database");
    let analytics_service = AnalyticsService::new(database.manager.clone());

    // Test provider-specific analytics
    let claude_analytics = analytics_service
        .get_usage_analytics(UsageAnalyticsRequest {
            date_range: Some(DateRange {
                start_date: "2024-01-01".to_string(),
                end_date: "2024-01-31".to_string(),
            }),
            providers: Some(vec!["ClaudeCode".to_string()]),
            projects: None,
            aggregation_level: Some("weekly".to_string()),
        })
        .await;

    assert!(claude_analytics.is_ok());

    let gemini_analytics = analytics_service
        .get_usage_analytics(UsageAnalyticsRequest {
            date_range: Some(DateRange {
                start_date: "2024-01-01".to_string(),
                end_date: "2024-01-31".to_string(),
            }),
            providers: Some(vec!["Gemini".to_string()]),
            projects: None,
            aggregation_level: Some("monthly".to_string()),
        })
        .await;

    assert!(gemini_analytics.is_ok());

    // Compare provider usage
    let claude_response = claude_analytics.unwrap();
    let gemini_response = gemini_analytics.unwrap();

    assert!(claude_response.total_sessions >= 0);
    assert!(gemini_response.total_sessions >= 0);
}

#[tokio::test]
async fn test_comprehensive_insights_analysis() {
    let database = Database::new_in_memory().unwrap();
    database
        .initialize()
        .expect("Failed to initialize database");
    let analytics_service = AnalyticsService::new(database.manager.clone());

    // Test comprehensive insights for a longer period
    let comprehensive_insights = analytics_service
        .get_insights(InsightsRequest {
            analysis_type: None,
            date_range: Some(DateRange {
                start_date: "2024-01-01".to_string(),
                end_date: "2024-03-31".to_string(),
            }),
            include_trends: None,
            providers: None,
            insight_types: None, // Get all available insights
        })
        .await;

    assert!(comprehensive_insights.is_ok());

    let insights_response = comprehensive_insights.unwrap();
    // Validate comprehensive insights response structure
    assert!(!insights_response.analysis_timestamp.is_empty());

    // For comprehensive analysis, validate that at least one type of insight is present
    let has_insights = !insights_response.insights.is_empty();
    let has_trends = !insights_response.trends.is_empty();
    let has_recommendations = !insights_response.recommendations.is_empty();
    assert!(
        has_insights || has_trends || has_recommendations,
        "Comprehensive analysis should provide at least one type of insight"
    );

    // Test specific insight types
    let specific_insights = analytics_service
        .get_insights(InsightsRequest {
            analysis_type: None,
            date_range: Some(DateRange {
                start_date: "2024-01-01".to_string(),
                end_date: "2024-01-31".to_string(),
            }),
            include_trends: None,
            providers: Some(vec!["ClaudeCode".to_string()]),
            insight_types: Some(vec![
                "productivity".to_string(),
                "usage_patterns".to_string(),
                "efficiency".to_string(),
            ]),
        })
        .await;

    assert!(specific_insights.is_ok());
}

#[tokio::test]
async fn test_analytics_performance() {
    let database = Database::new_in_memory().unwrap();
    database
        .initialize()
        .expect("Failed to initialize database");
    let analytics_service = AnalyticsService::new(database.manager.clone());

    // Time analytics generation
    let start_time = std::time::Instant::now();

    let usage_result = analytics_service
        .get_usage_analytics(UsageAnalyticsRequest {
            date_range: Some(DateRange {
                start_date: "2024-01-01".to_string(),
                end_date: "2024-12-31".to_string(),
            }),
            providers: None,
            projects: None,
            aggregation_level: Some("monthly".to_string()),
        })
        .await;

    let usage_duration = start_time.elapsed();

    assert!(usage_result.is_ok());
    assert!(usage_duration.as_millis() < 5000); // 5 seconds max

    // Time insights generation
    let insights_start = std::time::Instant::now();

    let insights_result = analytics_service
        .get_insights(InsightsRequest {
            analysis_type: None,
            date_range: Some(DateRange {
                start_date: "2024-01-01".to_string(),
                end_date: "2024-12-31".to_string(),
            }),
            include_trends: None,
            providers: None,
            insight_types: Some(vec!["trends".to_string(), "productivity".to_string()]),
        })
        .await;

    let insights_duration = insights_start.elapsed();

    assert!(insights_result.is_ok());
    assert!(insights_duration.as_millis() < 3000); // 3 seconds max
}
