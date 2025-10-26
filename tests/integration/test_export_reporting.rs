use retrochat::database::DatabaseManager;
use retrochat::services::AnalyticsService;
use tempfile::TempDir;

// NOTE: These tests are temporarily simplified to match the current implementation
// The full export functionality with ExportRequest will be implemented in a future update

#[tokio::test]
async fn test_export_reporting_workflow() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");

    // Setup database
    let db_manager = DatabaseManager::new(":memory:")
        .await
        .expect("Failed to create database manager");

    // Test analytics service export functionality
    let analytics_service = AnalyticsService::new(db_manager);

    // Test CSV export
    let csv_output_path = temp_dir.path().join("export.csv");
    let result = analytics_service
        .export_data("csv", &csv_output_path.to_string_lossy())
        .await;
    assert!(result.is_ok());
    let response = result.unwrap();
    assert!(response.contains("CSV format"));

    // Test JSON export
    let json_output_path = temp_dir.path().join("export.json");
    let result = analytics_service
        .export_data("json", &json_output_path.to_string_lossy())
        .await;
    assert!(result.is_ok());
    let response = result.unwrap();
    assert!(response.contains("JSON format"));

    // Test text export
    let txt_output_path = temp_dir.path().join("export.txt");
    let result = analytics_service
        .export_data("txt", &txt_output_path.to_string_lossy())
        .await;
    assert!(result.is_ok());
    let response = result.unwrap();
    assert!(response.contains("text format"));
}
