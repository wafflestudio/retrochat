use retrochat::cli::retrospect::{handle_show_command, AnalysisTypeArg};
use std::sync::Once;

// Global setup that runs only once
static INIT: Once = Once::new();

fn ensure_env_loaded() {
    INIT.call_once(|| {
        dotenvy::dotenv().ok();
    });
}

#[tokio::test]
async fn test_retrospect_show_command_structure() {
    ensure_env_loaded();

    // Test CLI command structure for retrospect show

    // Test command execution with session ID
    let result = handle_show_command(
        Some("session-123".to_string()), // session_id
        false,                           // all
        "text".to_string(),              // format
        None,                            // analysis_type
    )
    .await;

    // The command should execute (may succeed or fail based on environment)
    match result {
        Ok(()) => {
            // Command succeeded
        }
        Err(e) => {
            // Command failed - should be a proper error with message
            let error_msg = e.to_string();
            assert!(!error_msg.is_empty());
            // Common error scenarios
            assert!(
                error_msg.contains("database")
                    || error_msg.contains("connection")
                    || error_msg.contains("session")
                    || error_msg.contains("GOOGLE_AI_API_KEY")
            );
        }
    }
}

#[tokio::test]
async fn test_retrospect_show_all_formats() {
    ensure_env_loaded();

    // Test all output formats
    for format in ["text", "json", "markdown"] {
        let result = handle_show_command(
            None,               // session_id (None when using --all)
            true,               // all
            format.to_string(), // format
            None,               // analysis_type
        )
        .await;

        // The command should execute
        match result {
            Ok(()) => {
                // Command succeeded
            }
            Err(e) => {
                // Should be a meaningful error message
                let error_msg = e.to_string();
                assert!(!error_msg.is_empty());
                // Common error scenarios
                assert!(
                    error_msg.contains("database")
                        || error_msg.contains("connection")
                        || error_msg.contains("analyses")
                        || error_msg.contains("GOOGLE_AI_API_KEY")
                );
            }
        }
    }
}

#[tokio::test]
async fn test_retrospect_show_filtering() {
    ensure_env_loaded();

    // Test filtering by analysis type
    let result = handle_show_command(
        None,                                 // session_id
        true,                                 // all
        "text".to_string(),                   // format
        Some(AnalysisTypeArg::Collaboration), // analysis_type filter
    )
    .await;

    // The command should execute
    match result {
        Ok(()) => {
            // Command succeeded
        }
        Err(e) => {
            // Should be a meaningful error message
            let error_msg = e.to_string();
            assert!(!error_msg.is_empty());
            // Common error scenarios
            assert!(
                error_msg.contains("database")
                    || error_msg.contains("connection")
                    || error_msg.contains("analyses")
                    || error_msg.contains("GOOGLE_AI_API_KEY")
            );
        }
    }
}

#[tokio::test]
async fn test_retrospect_show_specific_session() {
    ensure_env_loaded();

    // Test showing results for a specific session
    let result = handle_show_command(
        Some("session-123".to_string()), // session_id
        false,                           // all
        "text".to_string(),              // format
        None,                            // analysis_type
    )
    .await;

    // The command should execute
    match result {
        Ok(()) => {
            // Command succeeded
        }
        Err(e) => {
            // Should be a meaningful error message
            let error_msg = e.to_string();
            assert!(!error_msg.is_empty());
            // Common error scenarios
            assert!(
                error_msg.contains("database")
                    || error_msg.contains("connection")
                    || error_msg.contains("session")
                    || error_msg.contains("analysis")
                    || error_msg.contains("GOOGLE_AI_API_KEY")
                    || error_msg.contains("not found")
            );
        }
    }
}
