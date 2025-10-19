use retrochat::cli::{Cli, Commands};
use retrochat::models::Provider;

#[test]
fn test_add_command_structure() {
    // Test that the Add command has the correct structure
    let add_cmd = Commands::Add {
        path: None,
        providers: vec![],
        overwrite: false,
    };

    match add_cmd {
        Commands::Add {
            path,
            providers,
            overwrite,
        } => {
            assert!(path.is_none());
            assert_eq!(providers.len(), 0);
            assert!(!overwrite);
        }
        _ => panic!("Expected Add command"),
    }
}

#[test]
fn test_add_command_with_path() {
    let add_cmd = Commands::Add {
        path: Some("/test/path".to_string()),
        providers: vec![],
        overwrite: false,
    };

    match add_cmd {
        Commands::Add { path, .. } => {
            assert_eq!(path, Some("/test/path".to_string()));
        }
        _ => panic!("Expected Add command"),
    }
}

#[test]
fn test_add_command_with_providers() {
    let add_cmd = Commands::Add {
        path: None,
        providers: vec![Provider::ClaudeCode, Provider::CursorAgent],
        overwrite: false,
    };

    match add_cmd {
        Commands::Add { providers, .. } => {
            assert_eq!(providers.len(), 2);
            assert_eq!(providers[0], Provider::ClaudeCode);
            assert_eq!(providers[1], Provider::CursorAgent);
        }
        _ => panic!("Expected Add command"),
    }
}

#[test]
fn test_add_command_with_overwrite() {
    let add_cmd = Commands::Add {
        path: None,
        providers: vec![],
        overwrite: true,
    };

    match add_cmd {
        Commands::Add { overwrite, .. } => {
            assert!(overwrite);
        }
        _ => panic!("Expected Add command"),
    }
}

#[test]
fn test_stats_command() {
    let stats_cmd = Commands::Stats;

    match stats_cmd {
        Commands::Stats => {
            // Stats command has no fields
            assert!(true);
        }
        _ => panic!("Expected Stats command"),
    }
}

#[test]
fn test_search_command_structure() {
    let search_cmd = Commands::Search {
        query: "test query".to_string(),
        limit: Some(10),
    };

    match search_cmd {
        Commands::Search { query, limit } => {
            assert_eq!(query, "test query");
            assert_eq!(limit, Some(10));
        }
        _ => panic!("Expected Search command"),
    }
}

#[test]
fn test_search_command_without_limit() {
    let search_cmd = Commands::Search {
        query: "test".to_string(),
        limit: None,
    };

    match search_cmd {
        Commands::Search { query, limit } => {
            assert_eq!(query, "test");
            assert!(limit.is_none());
        }
        _ => panic!("Expected Search command"),
    }
}

#[test]
fn test_review_command_structure() {
    let review_cmd = Commands::Review {
        session_id: Some("session-123".to_string()),
        analysis_type: None,
    };

    match review_cmd {
        Commands::Review {
            session_id,
            analysis_type,
        } => {
            assert_eq!(session_id, Some("session-123".to_string()));
            assert!(analysis_type.is_none());
        }
        _ => panic!("Expected Review command"),
    }
}

#[test]
fn test_setup_command() {
    let setup_cmd = Commands::Setup;

    match setup_cmd {
        Commands::Setup => {
            // Setup command has no fields
            assert!(true);
        }
        _ => panic!("Expected Setup command"),
    }
}

#[test]
fn test_cli_optional_command() {
    // Test that Cli can have None command
    let cli = Cli { command: None };

    assert!(cli.command.is_none(), "Command should be optional");
}

#[test]
fn test_cli_with_command() {
    let cli = Cli {
        command: Some(Commands::Stats),
    };

    assert!(cli.command.is_some(), "Command should be present");
    match cli.command {
        Some(Commands::Stats) => assert!(true),
        _ => panic!("Expected Stats command"),
    }
}
