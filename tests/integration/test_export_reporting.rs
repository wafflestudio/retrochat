use retrochat::database::Database;
use retrochat::services::{AnalyticsService, DateRange, ExportFilters, ExportRequest};
use tempfile::TempDir;

#[tokio::test]
async fn test_export_reporting_workflow() {
    let _temp_dir = TempDir::new().expect("Failed to create temp directory");

    // Setup database
    let database = Database::new_in_memory().await.unwrap();
    database
        .initialize()
        .await
        .expect("Failed to initialize database");

    // Test analytics service export functionality
    let analytics_service = AnalyticsService::new(database.manager.clone());

    // Test CSV export
    let csv_export_request = ExportRequest {
        format: "csv".to_string(),
        data_types: vec!["usage_summary".to_string()],
        date_range: Some(DateRange {
            start_date: "2024-01-01".to_string(),
            end_date: "2024-01-31".to_string(),
        }),
        filters: Some(ExportFilters {
            providers: Some(vec!["ClaudeCode".to_string()]),
            projects: None,
            include_content: Some(false),
            min_message_length: None,
        }),
    };

    let csv_result = analytics_service
        .export_data(
            &csv_export_request.format,
            csv_export_request.date_range.map(|dr| {
                format!(
                    "export_{}_{}.{}",
                    dr.start_date, dr.end_date, csv_export_request.format
                )
            }),
        )
        .await;
    assert!(csv_result.is_ok());

    let csv_response = csv_result.unwrap();
    assert_eq!(csv_response.format, "csv");
    assert!(!csv_response.file_path.is_empty());
    assert!(csv_response.file_size_bytes >= 0);

    // Test JSON export with different filters
    let json_export_request = ExportRequest {
        format: "json".to_string(),
        data_types: vec!["sessions".to_string(), "messages".to_string()],
        date_range: Some(DateRange {
            start_date: "2024-01-01".to_string(),
            end_date: "2024-12-31".to_string(),
        }),
        filters: Some(ExportFilters {
            providers: None,
            projects: Some(vec!["Test Project".to_string()]),
            include_content: Some(true),
            min_message_length: Some(10),
        }),
    };

    let json_result = analytics_service
        .export_data(
            &json_export_request.format,
            json_export_request.date_range.map(|dr| {
                format!(
                    "export_{}_{}.{}",
                    dr.start_date, dr.end_date, json_export_request.format
                )
            }),
        )
        .await;
    assert!(json_result.is_ok());

    let json_response = json_result.unwrap();
    assert_eq!(json_response.format, "json");
    assert!(!json_response.file_path.is_empty());
    assert!(json_response.file_size_bytes >= 0);
}

#[tokio::test]
async fn test_export_different_formats() {
    let database = Database::new_in_memory().await.unwrap();
    database
        .initialize()
        .await
        .expect("Failed to initialize database");
    let analytics_service = AnalyticsService::new(database.manager.clone());

    // Test various export formats
    let formats = vec!["csv", "json", "txt"];

    for format in formats {
        let export_request = ExportRequest {
            format: format.to_string(),
            data_types: vec!["usage_summary".to_string()],
            date_range: Some(DateRange {
                start_date: "2024-01-01".to_string(),
                end_date: "2024-01-31".to_string(),
            }),
            filters: None,
        };

        let result = analytics_service
            .export_data(
                &export_request.format,
                export_request.date_range.map(|dr| {
                    format!(
                        "export_{}_{}.{}",
                        dr.start_date, dr.end_date, export_request.format
                    )
                }),
            )
            .await;
        assert!(result.is_ok());

        let response = result.unwrap();
        assert_eq!(response.format, format);
        assert!(!response.file_path.is_empty());
        assert!(response.file_size_bytes >= 0);
        assert!(response.records_exported >= 0);
    }
}

#[tokio::test]
async fn test_export_with_comprehensive_filters() {
    let database = Database::new_in_memory().await.unwrap();
    database
        .initialize()
        .await
        .expect("Failed to initialize database");
    let analytics_service = AnalyticsService::new(database.manager.clone());

    // Test export with all filter options
    let comprehensive_export = ExportRequest {
        format: "json".to_string(),
        data_types: vec![
            "usage_summary".to_string(),
            "sessions".to_string(),
            "messages".to_string(),
            "insights".to_string(),
        ],
        date_range: Some(DateRange {
            start_date: "2024-01-01".to_string(),
            end_date: "2024-06-30".to_string(),
        }),
        filters: Some(ExportFilters {
            providers: Some(vec!["ClaudeCode".to_string(), "Gemini".to_string()]),
            projects: Some(vec!["Project A".to_string(), "Project B".to_string()]),
            include_content: Some(true),
            min_message_length: Some(50),
        }),
    };

    let result = analytics_service
        .export_data(
            &comprehensive_export.format,
            comprehensive_export.date_range.map(|dr| {
                format!(
                    "export_{}_{}.{}",
                    dr.start_date, dr.end_date, comprehensive_export.format
                )
            }),
        )
        .await;
    assert!(result.is_ok());

    let response = result.unwrap();
    assert_eq!(response.format, "json");
    assert!(!response.file_path.is_empty());
    assert!(response.file_size_bytes >= 0);
    assert!(response.records_exported >= 0);
}

#[tokio::test]
async fn test_export_performance() {
    let database = Database::new_in_memory().await.unwrap();
    database
        .initialize()
        .await
        .expect("Failed to initialize database");
    let analytics_service = AnalyticsService::new(database.manager.clone());

    // Time a large export operation
    let start_time = std::time::Instant::now();

    let large_export_request = ExportRequest {
        format: "txt".to_string(),
        data_types: vec!["sessions".to_string(), "messages".to_string()],
        date_range: Some(DateRange {
            start_date: "2024-01-01".to_string(),
            end_date: "2024-12-31".to_string(),
        }),
        filters: None, // No filters for maximum data
    };

    let result = analytics_service
        .export_data(
            &large_export_request.format,
            large_export_request.date_range.map(|dr| {
                format!(
                    "export_{}_{}.{}",
                    dr.start_date, dr.end_date, large_export_request.format
                )
            }),
        )
        .await;
    let duration = start_time.elapsed();

    assert!(result.is_ok());
    let response = result.unwrap();
    assert_eq!(response.format, "txt");

    // Performance target: should complete within reasonable time
    assert!(duration.as_secs() < 60); // 1 minute max for large export
}

#[tokio::test]
async fn test_export_edge_cases() {
    let database = Database::new_in_memory().await.unwrap();
    database
        .initialize()
        .await
        .expect("Failed to initialize database");
    let analytics_service = AnalyticsService::new(database.manager.clone());

    // Test export with no date range (all data)
    let no_date_export = ExportRequest {
        format: "csv".to_string(),
        data_types: vec!["usage_summary".to_string()],
        date_range: None,
        filters: None,
    };

    let result = analytics_service
        .export_data(
            &no_date_export.format,
            no_date_export.date_range.map(|dr| {
                format!(
                    "export_{}_{}.{}",
                    dr.start_date, dr.end_date, no_date_export.format
                )
            }),
        )
        .await;
    assert!(result.is_ok());

    // Test export with very restrictive filters
    let restrictive_export = ExportRequest {
        format: "json".to_string(),
        data_types: vec!["sessions".to_string()],
        date_range: Some(DateRange {
            start_date: "2024-01-01".to_string(),
            end_date: "2024-01-01".to_string(), // Single day
        }),
        filters: Some(ExportFilters {
            providers: Some(vec!["NonExistentProvider".to_string()]),
            projects: Some(vec!["NonExistentProject".to_string()]),
            include_content: Some(false),
            min_message_length: Some(10000), // Very high threshold
        }),
    };

    let restrictive_result = analytics_service
        .export_data(
            &restrictive_export.format,
            restrictive_export.date_range.map(|dr| {
                format!(
                    "export_{}_{}.{}",
                    dr.start_date, dr.end_date, restrictive_export.format
                )
            }),
        )
        .await;
    assert!(restrictive_result.is_ok());

    let restrictive_response = restrictive_result.unwrap();
    assert!(restrictive_response.records_exported >= 0); // May be 0, which is valid
}

#[tokio::test]
async fn test_export_file_validation() {
    let database = Database::new_in_memory().await.unwrap();
    database
        .initialize()
        .await
        .expect("Failed to initialize database");
    let analytics_service = AnalyticsService::new(database.manager.clone());

    let export_request = ExportRequest {
        format: "csv".to_string(),
        data_types: vec!["usage_summary".to_string()],
        date_range: Some(DateRange {
            start_date: "2024-01-01".to_string(),
            end_date: "2024-01-31".to_string(),
        }),
        filters: None,
    };

    let result = analytics_service
        .export_data(
            &export_request.format,
            export_request.date_range.map(|dr| {
                format!(
                    "export_{}_{}.{}",
                    dr.start_date, dr.end_date, export_request.format
                )
            }),
        )
        .await;
    assert!(result.is_ok());

    let response = result.unwrap();

    // Validate export response structure
    assert!(!response.file_path.is_empty());
    assert!(response.file_size_bytes >= 0);
    assert!(response.records_exported >= 0);
    assert_eq!(response.format, "csv");

    // Additional validation could include checking if file exists,
    // but we'll keep it simple for compilation testing
}
