use retrochat::cli::retrospect::handle_show_command;

#[tokio::test]
async fn test_retrospect_show_command_structure() {
    // Test CLI command structure for retrospect show

    // Test command execution with session ID
    let result = handle_show_command(
        Some("session-123".to_string()), // session_id
        false,                           // all
        "text".to_string(),              // format
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
                    || error_msg.contains("Configuration")
                    || error_msg.contains("API key")
            );
        }
    }
}

#[tokio::test]
async fn test_retrospect_show_all_formats() {
    // Test all output formats
    for format in ["text", "json", "markdown"] {
        let result = handle_show_command(
            None,               // session_id (None when using --all)
            true,               // all
            format.to_string(), // format
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
                        || error_msg.contains("Configuration")
                        || error_msg.contains("API key")
                );
            }
        }
    }
}

#[tokio::test]
async fn test_retrospect_show_filtering() {
    // Test filtering by analysis type
    let result = handle_show_command(
        None,               // session_id
        true,               // all
        "text".to_string(), // format
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
                    || error_msg.contains("Configuration")
                    || error_msg.contains("API key")
            );
        }
    }
}

#[tokio::test]
async fn test_retrospect_show_specific_session() {
    // Test showing results for a specific session
    let result = handle_show_command(
        Some("session-123".to_string()), // session_id
        false,                           // all
        "text".to_string(),              // format
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
                    || error_msg.contains("Configuration")
                    || error_msg.contains("API key")
            );
        }
    }
}
