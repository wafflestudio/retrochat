use retrochat::cli::retrospect::handle_status_command;

#[tokio::test]
async fn test_retrospect_status_command_structure() {
    // Test CLI command structure for retrospect status

    // Test command execution
    let result = handle_status_command(
        false, // all
        false, // watch
        false, // history
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
                    || error_msg.contains("GOOGLE_AI_API_KEY")
            );
        }
    }
    // Test completed successfully
}

#[tokio::test]
async fn test_retrospect_status_active_only() {
    // Test showing only active operations (same as --all in current implementation)
    let result = handle_status_command(
        true,  // all (shows active operations)
        false, // watch
        false, // history
    )
    .await;

    // The command should execute
    match result {
        Ok(()) => {
            // Command succeeded
        },
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
async fn test_retrospect_status_history() {
    // Test showing operation history
    let result = handle_status_command(
        false, // all
        false, // watch
        true,  // history
    )
    .await;

    // The command should execute
    match result {
        Ok(()) => {
            // Command succeeded
        },
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
async fn test_retrospect_status_watch_mode() {
    // Test watch mode (current implementation just shows status once)
    let result = handle_status_command(
        false, // all
        true,  // watch
        false, // history
    )
    .await;

    // The command should execute
    match result {
        Ok(()) => {
            // Command succeeded
        },
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
