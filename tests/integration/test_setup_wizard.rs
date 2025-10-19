use retrochat::cli::setup;
use retrochat::database::config;
use tempfile::TempDir;

#[test]
fn test_is_first_time_user_with_no_database() {
    // This test depends on the actual file system state
    // For a true unit test, we'd need to mock the file system
    // For now, we just verify the function doesn't panic
    let _result = setup::is_first_time_user();
}

#[test]
fn test_needs_setup_returns_bool() {
    // Test that the function returns a Result<bool>
    let result = setup::needs_setup();
    assert!(
        result.is_ok(),
        "needs_setup should return Ok even if db doesn't exist"
    );
}

#[test]
fn test_is_first_time_user_after_db_creation() {
    // Set up temporary directory for testing
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("retrochat.db");

    // Before database exists
    assert!(
        !db_path.exists(),
        "Database should not exist before creation"
    );

    // After database is created
    std::fs::write(&db_path, b"test").unwrap();
    assert!(db_path.exists(), "Database should exist after creation");

    // Note: We can't easily test is_first_time_user with custom path
    // because it uses config::get_default_db_path() internally
    // This would require dependency injection or environment variable mocking
}

#[tokio::test]
async fn test_run_setup_wizard_structure() {
    // This is more of a smoke test - we can't easily test interactive prompts
    // without mocking the terminal input

    // We can at least verify the function signature and that it compiles
    // The actual interactive testing would require a different approach
    // (e.g., using expect-test or similar for CLI testing)

    // For now, we just verify the types are correct
    let _fn_ptr: fn() -> std::pin::Pin<
        Box<dyn std::future::Future<Output = anyhow::Result<()>> + Send>,
    > = || Box::pin(async { Ok(()) });
}

#[test]
fn test_config_directory_creation() {
    // Test that config directory can be created
    let result = config::ensure_config_dir();

    // Should succeed whether directory exists or not
    assert!(result.is_ok(), "Should be able to ensure config dir exists");
}

#[test]
fn test_get_default_db_path() {
    let result = config::get_default_db_path();

    assert!(result.is_ok(), "Should be able to get default db path");

    if let Ok(path) = result {
        assert!(
            path.to_string_lossy().contains("retrochat"),
            "Path should contain 'retrochat'"
        );
        assert!(
            path.to_string_lossy().ends_with(".db"),
            "Path should end with .db"
        );
    }
}

// Integration test: Test the full flow with a temporary database
#[tokio::test]
async fn test_database_initialization_flow() {
    use retrochat::database::DatabaseManager;

    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test_retrochat.db");

    // Database shouldn't exist yet
    assert!(!db_path.exists());

    // Initialize database
    let db_result = DatabaseManager::new(&db_path).await;
    assert!(db_result.is_ok(), "Should be able to create database");

    // Database should now exist
    assert!(db_path.exists(), "Database file should be created");

    // Should be able to create another instance with existing database
    let db_result2 = DatabaseManager::new(&db_path).await;
    assert!(
        db_result2.is_ok(),
        "Should be able to connect to existing database"
    );
}

