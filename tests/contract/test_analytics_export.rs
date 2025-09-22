use retrochat::database::connection::DatabaseManager;
use retrochat::services::{AnalyticsService, DateRange, ExportFilters, ExportRequest};

#[tokio::test]
async fn test_export_csv_basic() {
    let db_manager = DatabaseManager::new(":memory:").unwrap();
    let service = AnalyticsService::new(db_manager);
    let request = ExportRequest {
        format: "csv".to_string(),
        data_types: vec!["usage_summary".to_string()],
        date_range: Some(DateRange {
            start_date: "2024-01-01".to_string(),
            end_date: "2024-01-31".to_string(),
        }),
        filters: None,
    };

    let result = service
        .export_data(
            &request.format,
            request.date_range.map(|dr| {
                format!(
                    "export_{}_{}.{}",
                    dr.start_date, dr.end_date, request.format
                )
            }),
        )
        .await;
    assert!(result.is_ok());

    let response = result.unwrap();
    assert_eq!(response.format, "csv");
    assert!(!response.file_path.is_empty());
    assert!(response.file_size_bytes >= 0);
}

#[tokio::test]
async fn test_export_json_with_filters() {
    let db_manager = DatabaseManager::new(":memory:").unwrap();
    let service = AnalyticsService::new(db_manager);
    let request = ExportRequest {
        format: "json".to_string(),
        data_types: vec!["sessions".to_string(), "messages".to_string()],
        date_range: Some(DateRange {
            start_date: "2024-01-01".to_string(),
            end_date: "2024-12-31".to_string(),
        }),
        filters: Some(ExportFilters {
            providers: Some(vec!["ClaudeCode".to_string()]),
            projects: Some(vec!["Test Project".to_string()]),
            include_content: Some(true),
            min_message_length: Some(50),
        }),
    };

    let result = service
        .export_data(
            &request.format,
            request.date_range.map(|dr| {
                format!(
                    "export_{}_{}.{}",
                    dr.start_date, dr.end_date, request.format
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
async fn test_export_multiple_data_types() {
    let db_manager = DatabaseManager::new(":memory:").unwrap();
    let service = AnalyticsService::new(db_manager);
    let request = ExportRequest {
        format: "json".to_string(),
        data_types: vec![
            "usage_summary".to_string(),
            "sessions".to_string(),
            "messages".to_string(),
            "insights".to_string(),
        ],
        date_range: None, // All available data
        filters: None,
    };

    let result = service
        .export_data(
            &request.format,
            request.date_range.map(|dr| {
                format!(
                    "export_{}_{}.{}",
                    dr.start_date, dr.end_date, request.format
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
async fn test_export_parquet_format() {
    let db_manager = DatabaseManager::new(":memory:").unwrap();
    let service = AnalyticsService::new(db_manager);
    let request = ExportRequest {
        format: "txt".to_string(),
        data_types: vec!["sessions".to_string()],
        date_range: Some(DateRange {
            start_date: "2024-01-01".to_string(),
            end_date: "2024-06-30".to_string(),
        }),
        filters: Some(ExportFilters {
            providers: None,
            projects: None,
            include_content: Some(false),
            min_message_length: None,
        }),
    };

    let result = service
        .export_data(
            &request.format,
            request.date_range.map(|dr| {
                format!(
                    "export_{}_{}.{}",
                    dr.start_date, dr.end_date, request.format
                )
            }),
        )
        .await;
    assert!(result.is_ok());

    let response = result.unwrap();
    assert_eq!(response.format, "txt");
    assert!(!response.file_path.is_empty());
    assert!(response.file_size_bytes >= 0);
}
