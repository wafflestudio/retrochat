use retrochat_core::cli::{Cli, Commands};
use retrochat_core::models::Provider;

#[test]
fn test_sync_command_structure() {
    // Test that the Sync command has the correct structure
    let sync_cmd = Commands::Sync {
        providers: vec![],
        path: None,
        overwrite: false,
        watch: false,
        verbose: false,
    };

    match sync_cmd {
        Commands::Sync {
            providers,
            path,
            overwrite,
            watch,
            verbose,
        } => {
            assert_eq!(providers.len(), 0);
            assert!(path.is_none());
            assert!(!overwrite);
            assert!(!watch);
            assert!(!verbose);
        }
        _ => panic!("Expected Sync command"),
    }
}

#[test]
fn test_sync_command_with_path() {
    let sync_cmd = Commands::Sync {
        providers: vec![],
        path: Some("/test/path".to_string()),
        overwrite: false,
        watch: false,
        verbose: false,
    };

    match sync_cmd {
        Commands::Sync { path, .. } => {
            assert_eq!(path, Some("/test/path".to_string()));
        }
        _ => panic!("Expected Sync command"),
    }
}

#[test]
fn test_sync_command_with_providers() {
    let sync_cmd = Commands::Sync {
        providers: vec![Provider::ClaudeCode, Provider::GeminiCLI],
        path: None,
        overwrite: false,
        watch: false,
        verbose: false,
    };

    match sync_cmd {
        Commands::Sync { providers, .. } => {
            assert_eq!(providers.len(), 2);
            assert_eq!(providers[0], Provider::ClaudeCode);
            assert_eq!(providers[1], Provider::GeminiCLI);
        }
        _ => panic!("Expected Sync command"),
    }
}

#[test]
fn test_sync_command_with_overwrite() {
    let sync_cmd = Commands::Sync {
        providers: vec![],
        path: None,
        overwrite: true,
        watch: false,
        verbose: false,
    };

    match sync_cmd {
        Commands::Sync { overwrite, .. } => {
            assert!(overwrite);
        }
        _ => panic!("Expected Sync command"),
    }
}

#[test]
fn test_sync_command_watch_mode() {
    let sync_cmd = Commands::Sync {
        providers: vec![Provider::ClaudeCode],
        path: None,
        overwrite: false,
        watch: true,
        verbose: false,
    };

    match sync_cmd {
        Commands::Sync { watch, .. } => {
            assert!(watch);
        }
        _ => panic!("Expected Sync command"),
    }
}

#[test]
fn test_sync_command_verbose() {
    let sync_cmd = Commands::Sync {
        providers: vec![],
        path: None,
        overwrite: false,
        watch: true,
        verbose: true,
    };

    match sync_cmd {
        Commands::Sync { verbose, .. } => {
            assert!(verbose);
        }
        _ => panic!("Expected Sync command"),
    }
}

#[test]
fn test_search_command_structure() {
    let search_cmd = Commands::Search {
        query: "test query".to_string(),
        limit: Some(10),
        since: None,
        until: None,
    };

    match search_cmd {
        Commands::Search {
            query,
            limit,
            since,
            until,
        } => {
            assert_eq!(query, "test query");
            assert_eq!(limit, Some(10));
            assert!(since.is_none());
            assert!(until.is_none());
        }
        _ => panic!("Expected Search command"),
    }
}

#[test]
fn test_search_command_without_limit() {
    let search_cmd = Commands::Search {
        query: "test".to_string(),
        limit: None,
        since: None,
        until: None,
    };

    match search_cmd {
        Commands::Search {
            query,
            limit,
            since,
            until,
        } => {
            assert_eq!(query, "test");
            assert!(limit.is_none());
            assert!(since.is_none());
            assert!(until.is_none());
        }
        _ => panic!("Expected Search command"),
    }
}

#[test]
fn test_search_command_with_time_range() {
    let search_cmd = Commands::Search {
        query: "test".to_string(),
        limit: Some(10),
        since: Some("7 days ago".to_string()),
        until: Some("now".to_string()),
    };

    match search_cmd {
        Commands::Search {
            query,
            limit,
            since,
            until,
        } => {
            assert_eq!(query, "test");
            assert_eq!(limit, Some(10));
            assert_eq!(since, Some("7 days ago".to_string()));
            assert_eq!(until, Some("now".to_string()));
        }
        _ => panic!("Expected Search command"),
    }
}

#[test]
fn test_analysis_run_command_structure() {
    use retrochat_core::cli::AnalysisCommands;

    let analysis_cmd = Commands::Analysis {
        command: AnalysisCommands::Run {
            session_id: Some("session-123".to_string()),
            custom_prompt: None,
            all: false,
            background: false,
        },
    };

    match analysis_cmd {
        Commands::Analysis {
            command: AnalysisCommands::Run { session_id, .. },
        } => {
            assert_eq!(session_id, Some("session-123".to_string()));
        }
        _ => panic!("Expected Analysis Run command"),
    }
}

#[test]
fn test_cli_optional_command() {
    // Test that Cli can have None command
    let cli = Cli { command: None };

    assert!(cli.command.is_none(), "Command should be optional");
}

#[test]
fn test_cli_with_sync_command() {
    let cli = Cli {
        command: Some(Commands::Sync {
            providers: vec![],
            path: None,
            overwrite: false,
            watch: false,
            verbose: false,
        }),
    };

    assert!(cli.command.is_some(), "Command should be present");
    match cli.command {
        Some(Commands::Sync { .. }) => {}
        _ => panic!("Expected Sync command"),
    }
}
