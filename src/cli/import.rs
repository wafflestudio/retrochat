use anyhow::{Context, Result};
use crossterm::style::{Color, Stylize};
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::cli::help;
use crate::database::DatabaseManager;
use crate::models::provider::config::{
    ClaudeCodeConfig, CodexConfig, CursorAgentConfig, GeminiCliConfig,
};
use crate::models::Provider;
use crate::services::ImportService;

pub async fn handle_import_command(
    path: Option<String>,
    providers: Vec<Provider>,
    overwrite: bool,
    watch: bool,
) -> Result<()> {
    if watch {
        // Collect all paths to watch
        let mut watch_paths = Vec::new();

        // Add explicit path if provided
        if let Some(path_str) = &path {
            watch_paths.push(path_str.clone());
        }

        // Add provider directories if specified
        if !providers.is_empty() {
            let provider_paths = collect_provider_paths(&providers)?;
            watch_paths.extend(provider_paths);
        }

        if watch_paths.is_empty() {
            help::print_import_usage();
            return Err(anyhow::anyhow!(
                "No paths to watch. Provide --path or specify providers."
            ));
        }

        return watch_paths_for_changes(watch_paths).await;
    }

    // Non-watch mode: original behavior
    // Check if user provided a path
    if let Some(path_str) = path {
        return import_path(path_str, overwrite).await;
    }

    // Check if any providers are specified
    if !providers.is_empty() {
        return import_providers(providers, overwrite).await;
    }

    // No arguments provided - show help message
    help::print_import_usage();
    Err(anyhow::anyhow!("No import source specified"))
}

async fn import_path(path_str: String, overwrite: bool) -> Result<()> {
    let path = Path::new(&path_str);

    if !path.exists() {
        return Err(anyhow::anyhow!("Path does not exist: {path_str}"));
    }

    if path.is_file() {
        import_file(path_str, overwrite).await
    } else if path.is_dir() {
        import_batch(path_str, overwrite).await
    } else {
        Err(anyhow::anyhow!(
            "Path is neither a file nor a directory: {path_str}"
        ))
    }
}

async fn import_providers(providers: Vec<Provider>, overwrite: bool) -> Result<()> {
    // Expand "All" to all specific providers
    let expanded_providers = Provider::expand_all(providers);

    let mut imported_any = false;

    for provider in expanded_providers {
        match provider {
            Provider::All => {
                // Should not happen due to expansion above, but handle it anyway
                unreachable!("Provider::All should have been expanded")
            }
            Provider::ClaudeCode => {
                println!("Importing from Claude Code directories...");
                if let Err(e) = ClaudeCodeConfig::import_directories(overwrite, |path, ow| {
                    Box::pin(import_batch(path, ow))
                })
                .await
                {
                    eprintln!("Error importing Claude directories: {e}");
                } else {
                    imported_any = true;
                }
                println!();
            }
            Provider::GeminiCLI => {
                println!("Importing from Gemini directories...");
                if let Err(e) = GeminiCliConfig::import_directories(overwrite, |path, ow| {
                    Box::pin(import_batch(path, ow))
                })
                .await
                {
                    eprintln!("Error importing Gemini directories: {e}");
                } else {
                    imported_any = true;
                }
                println!();
            }
            Provider::Codex => {
                println!("Importing from Codex directories...");
                if let Err(e) = CodexConfig::import_directories(overwrite, |path, ow| {
                    Box::pin(import_batch(path, ow))
                })
                .await
                {
                    eprintln!("Error importing Codex directories: {e}");
                } else {
                    imported_any = true;
                }
                println!();
            }
            Provider::CursorAgent => {
                println!("Importing from Cursor directories...");
                if let Err(e) = CursorAgentConfig::import_directories(overwrite, |path, ow| {
                    Box::pin(import_batch(path, ow))
                })
                .await
                {
                    eprintln!("Error importing Cursor directories: {e}");
                } else {
                    imported_any = true;
                }
                println!();
            }
            Provider::Other(name) => {
                eprintln!("Unknown provider: {name}");
            }
        }
    }

    if imported_any {
        Ok(())
    } else {
        Err(anyhow::anyhow!("No providers were successfully imported"))
    }
}

async fn import_file(file_path: String, overwrite: bool) -> Result<()> {
    let path = Path::new(&file_path);

    println!("Importing file: {}", path.display());

    let db_path = crate::database::config::get_default_db_path()?;
    let db_manager = Arc::new(DatabaseManager::new(&db_path).await?);
    let import_service = ImportService::new(db_manager);

    // Detect provider
    let provider = crate::parsers::ParserRegistry::detect_provider(path)
        .ok_or_else(|| anyhow::anyhow!("Unsupported file format: {file_path}"))?;

    println!("Detected format: {provider}");

    if overwrite {
        println!("Overwrite mode: Will replace existing sessions");
    }

    let import_request = crate::services::ImportFileRequest {
        file_path: file_path.clone(),
        provider: Some(provider.to_string()),
        project_name: None,
        overwrite_existing: Some(overwrite),
    };

    let import_response = import_service
        .import_file(import_request)
        .await
        .with_context(|| format!("Failed to import file: {file_path}"))?;

    println!("Import completed:");
    println!(
        "  - {} sessions imported",
        import_response.sessions_imported
    );
    println!(
        "  - {} messages imported",
        import_response.messages_imported
    );

    if !import_response.warnings.is_empty() {
        println!("Warnings:");
        for warning in &import_response.warnings {
            println!("  - {warning}");
        }
    }

    Ok(())
}

async fn import_batch(directory: String, overwrite: bool) -> Result<()> {
    let path = Path::new(&directory);

    println!("Batch importing from directory: {}", path.display());

    if overwrite {
        println!("Overwrite mode: Will replace existing sessions");
    }

    let db_path = crate::database::config::get_default_db_path()?;
    let db_manager = Arc::new(DatabaseManager::new(&db_path).await?);
    let import_service = ImportService::new(db_manager);

    let batch_request = crate::services::BatchImportRequest {
        directory_path: directory.clone(),
        providers: None,
        project_name: None,
        overwrite_existing: Some(overwrite),
        recursive: Some(true),
    };

    let batch_response = import_service
        .import_batch(batch_request)
        .await
        .with_context(|| format!("Failed to batch import from directory: {}", path.display()))?;

    println!("\nBatch import completed:");
    println!(
        "  - {} files processed",
        batch_response.total_files_processed
    );
    println!(
        "  - {} files imported successfully",
        batch_response.successful_imports
    );
    println!(
        "  - {} sessions imported",
        batch_response.total_sessions_imported
    );
    println!(
        "  - {} messages imported",
        batch_response.total_messages_imported
    );

    if batch_response.failed_imports > 0 {
        println!(
            "  - {} files failed to import",
            batch_response.failed_imports
        );
        if !batch_response.errors.is_empty() {
            println!("Errors:");
            for error in &batch_response.errors {
                println!("  - {error}");
            }
        }
    }

    Ok(())
}

/// Collect all provider directories to watch
fn collect_provider_paths(providers: &[Provider]) -> Result<Vec<String>> {
    let expanded_providers = Provider::expand_all(providers.to_vec());
    let mut paths = Vec::new();

    for provider in expanded_providers {
        match provider {
            Provider::All => {
                unreachable!("Provider::All should have been expanded")
            }
            Provider::ClaudeCode => {
                let config = ClaudeCodeConfig::create();
                let dirs = config.get_import_directories();
                paths.extend(dirs);
            }
            Provider::GeminiCLI => {
                let config = GeminiCliConfig::create();
                let dirs = config.get_import_directories();
                paths.extend(dirs);
            }
            Provider::Codex => {
                let config = CodexConfig::create();
                let dirs = config.get_import_directories();
                paths.extend(dirs);
            }
            Provider::CursorAgent => {
                let config = CursorAgentConfig::create();
                let dirs = config.get_import_directories();
                paths.extend(dirs);
            }
            Provider::Other(name) => {
                eprintln!("Unknown provider: {name}");
            }
        }
    }

    Ok(paths)
}

/// Watch paths for file system changes and print events
async fn watch_paths_for_changes(paths: Vec<String>) -> Result<()> {
    use std::sync::mpsc::channel;

    println!("{}", "👁️  Starting file watcher...".with(Color::Cyan).bold());
    println!("{} {} path(s):", "📂".with(Color::Yellow), "Watching".bold());
    for path in &paths {
        println!("  {} {}", "└─".with(Color::DarkGrey), path.as_str().with(Color::Green));
    }
    println!("\n{} {}\n", "⌨️".with(Color::Blue), "Press Ctrl+C to stop watching.".with(Color::DarkGrey));

    let (tx, rx) = channel();

    let mut watcher: RecommendedWatcher = Watcher::new(
        move |res: Result<Event, notify::Error>| {
            if let Ok(event) = res {
                let _ = tx.send(event);
            }
        },
        notify::Config::default(),
    )?;

    // Watch all paths
    for path_str in &paths {
        let path = PathBuf::from(path_str);
        if !path.exists() {
            eprintln!("{} {} {}", "⚠️".with(Color::Yellow), "Warning:".with(Color::Yellow).bold(), format!("Path does not exist: {}", path_str).with(Color::DarkGrey));
            continue;
        }

        // Determine if we should watch recursively
        let mode = if path.is_dir() {
            RecursiveMode::Recursive
        } else {
            RecursiveMode::NonRecursive
        };

        watcher
            .watch(&path, mode)
            .with_context(|| format!("Failed to watch path: {path_str}"))?;
    }

    // Process events
    loop {
        match rx.recv() {
            Ok(event) => {
                print_event(&event);
            }
            Err(e) => {
                eprintln!("{} {} {}", "❌".with(Color::Red), "Watch error:".with(Color::Red).bold(), e.to_string().with(Color::DarkGrey));
                break;
            }
        }
    }

    Ok(())
}

/// Print a filesystem event
fn print_event(event: &Event) {
    let (emoji, event_kind, color) = match &event.kind {
        EventKind::Create(_) => ("✨", "CREATE", Color::Green),
        EventKind::Modify(_) => ("📝", "MODIFY", Color::Yellow),
        EventKind::Remove(_) => ("🗑️ ", "REMOVE", Color::Red),
        EventKind::Access(_) => ("👀", "ACCESS", Color::Blue),
        EventKind::Any => ("❓", "ANY", Color::DarkGrey),
        EventKind::Other => ("❓", "OTHER", Color::DarkGrey),
    };

    println!("{} {}", emoji, format!("[{}]", event_kind).with(color).bold());
    for path in &event.paths {
        let detection = detect_provider(path);

        // Choose color based on match type
        let provider_color = if detection.provider == "Unknown format" {
            Color::DarkGrey
        } else if detection.file_pattern_matched {
            Color::Magenta // Highlight if file pattern matched
        } else {
            Color::DarkGrey // Grey if only directory matched
        };

        // Build the provider display string
        let provider_display = if let Some(pattern) = &detection.matched_pattern {
            format!("{} ({})", detection.provider, pattern)
        } else {
            detection.provider.clone()
        };

        println!(
            "  {} {} {} {}",
            "→".with(Color::DarkGrey),
            path.display().to_string().with(Color::Cyan),
            "·".with(Color::DarkGrey),
            provider_display.with(provider_color)
        );
    }
}

/// Result of provider detection
#[derive(Debug, Clone)]
struct ProviderDetection {
    provider: String,
    file_pattern_matched: bool,
    matched_pattern: Option<String>,
}

/// Detect provider from file path using directory and file patterns
fn detect_provider(file_path: &Path) -> ProviderDetection {
    let file_name = file_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("");

    let parent_dir = file_path
        .parent()
        .and_then(|p| p.to_str())
        .unwrap_or("");

    // Create all provider configs
    let providers = vec![
        (ClaudeCodeConfig::create(), "Claude Code"),
        (GeminiCliConfig::create(), "Gemini CLI"),
        (CodexConfig::create(), "Codex"),
        (CursorAgentConfig::create(), "Cursor Agent"),
    ];

    // Priority 1: Check directory + file pattern match (most specific)
    for (config, provider_name) in &providers {
        if let Some(matched_pattern) = find_matching_pattern(&config.file_patterns, file_name) {
            // Check if directory also matches
            let dir_matches = check_directory_match(&config, parent_dir);

            if dir_matches {
                return ProviderDetection {
                    provider: provider_name.to_string(),
                    file_pattern_matched: true,
                    matched_pattern: Some(matched_pattern),
                };
            }
        }
    }

    // Priority 2: Check file pattern only (when directory doesn't match any provider)
    for (config, provider_name) in &providers {
        if let Some(matched_pattern) = find_matching_pattern(&config.file_patterns, file_name) {
            return ProviderDetection {
                provider: provider_name.to_string(),
                file_pattern_matched: true,
                matched_pattern: Some(matched_pattern),
            };
        }
    }

    // Priority 3: Check directory only match (no file pattern match)
    for (config, provider_name) in &providers {
        if check_directory_match(&config, parent_dir) {
            return ProviderDetection {
                provider: provider_name.to_string(),
                file_pattern_matched: false,
                matched_pattern: None,
            };
        }
    }

    ProviderDetection {
        provider: "Unknown format".to_string(),
        file_pattern_matched: false,
        matched_pattern: None,
    }
}

/// Check if the parent directory matches the provider's directories
fn check_directory_match(config: &crate::models::ProviderConfig, parent_dir: &str) -> bool {
    // Check default directory
    if let Some(default_dir) = config.default_directory() {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        let expanded_dir = default_dir.replace('~', &home);

        if parent_dir.contains(&expanded_dir) {
            return true;
        }
    }

    // Check all import directories (including env var dirs)
    let import_dirs = config.get_import_directories();
    for dir in import_dirs {
        if parent_dir.contains(&dir) {
            return true;
        }
    }

    false
}

/// Find which pattern matches the filename and return it
fn find_matching_pattern(patterns: &[String], file_name: &str) -> Option<String> {
    for pattern in patterns {
        // Simple glob pattern matching
        if pattern.contains('*') {
            let parts: Vec<&str> = pattern.split('*').collect();
            if parts.len() == 2 {
                if file_name.starts_with(parts[0]) && file_name.ends_with(parts[1]) {
                    return Some(pattern.clone());
                }
            }
        } else if file_name == pattern {
            // Exact match for non-wildcard patterns (e.g., "store.db" only matches "store.db", not "store.db-wal")
            return Some(pattern.clone());
        }
    }
    None
}
