use anyhow::{Context, Result};
use crossterm::style::{Color, Stylize};
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use similar::{ChangeTag, TextDiff};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use crate::models::provider::registry::ProviderRegistry;
use crate::models::Provider;
use crate::services::ParserService;

/// Result of provider detection
#[derive(Debug, Clone)]
pub struct ProviderDetection {
    pub provider: String,
    pub file_pattern_matched: bool,
    pub matched_pattern: Option<String>,
}

/// Collect all provider directories to watch
pub fn collect_provider_paths(providers: &[Provider]) -> Result<Vec<String>> {
    let expanded_providers = Provider::expand_all(providers.to_vec());
    let mut paths = Vec::new();

    let registry = ProviderRegistry::global();

    for provider in expanded_providers {
        match provider {
            Provider::All => {
                unreachable!("Provider::All should have been expanded")
            }
            Provider::ClaudeCode
            | Provider::GeminiCLI
            | Provider::Codex
            | Provider::CursorAgent => {
                if let Some(config) = registry.get_provider(&provider) {
                    let dirs = config.get_import_directories();
                    paths.extend(dirs);
                }
            }
            Provider::Other(name) => {
                eprintln!("Unknown provider: {name}");
            }
        }
    }

    Ok(paths)
}

/// Watch paths for file system changes and print events
pub async fn watch_paths_for_changes(paths: Vec<String>, verbose: bool) -> Result<()> {
    use std::sync::mpsc::channel;
    use tokio::sync::mpsc as tokio_mpsc;

    println!(
        "{}",
        "👁️  Starting file watcher...".with(Color::Cyan).bold()
    );
    if verbose {
        println!(
            "{} {}",
            "🔍".with(Color::Cyan),
            "Verbose mode: Will show diffs for JSON/JSONL files".with(Color::Cyan)
        );
    }
    println!(
        "{} {} path(s):",
        "📂".with(Color::Yellow),
        "Watching".bold()
    );
    for path in &paths {
        println!(
            "  {} {}",
            "└─".with(Color::DarkGrey),
            path.as_str().with(Color::Green)
        );
    }
    println!(
        "\n{} {}\n",
        "⌨️".with(Color::Blue),
        "Press Ctrl+C to stop watching.".with(Color::DarkGrey)
    );

    // File content cache for diff comparison
    let file_cache: Arc<Mutex<HashMap<PathBuf, String>>> = Arc::new(Mutex::new(HashMap::new()));

    // Create channels for file system events and parse requests
    let (tx, rx) = channel();
    let (parse_tx, mut parse_rx) = tokio_mpsc::unbounded_channel::<PathBuf>();

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
            eprintln!(
                "{} {} {}",
                "⚠️".with(Color::Yellow),
                "Warning:".with(Color::Yellow).bold(),
                format!("Path does not exist: {path_str}").with(Color::DarkGrey)
            );
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

    // Spawn a task to handle async parsing
    let parse_handle = tokio::spawn(async move {
        while let Some(path) = parse_rx.recv().await {
            parse_and_log_sessions_async(&path).await;
        }
    });

    // Process events
    loop {
        match rx.recv() {
            Ok(event) => {
                print_event(&event, verbose, &file_cache, parse_tx.clone());
            }
            Err(e) => {
                eprintln!(
                    "{} {} {}",
                    "❌".with(Color::Red),
                    "Watch error:".with(Color::Red).bold(),
                    e.to_string().with(Color::DarkGrey)
                );
                break;
            }
        }
    }

    // Clean up
    drop(parse_tx);
    let _ = parse_handle.await;

    Ok(())
}

/// Print a filesystem event
fn print_event(
    event: &Event,
    verbose: bool,
    file_cache: &Arc<Mutex<HashMap<PathBuf, String>>>,
    parse_tx: tokio::sync::mpsc::UnboundedSender<PathBuf>,
) {
    let (emoji, event_kind, color) = match &event.kind {
        EventKind::Create(_) => ("✨", "CREATE", Color::Green),
        EventKind::Modify(_) => ("📝", "MODIFY", Color::Yellow),
        EventKind::Remove(_) => ("🗑️ ", "REMOVE", Color::Red),
        EventKind::Access(_) => ("👀", "ACCESS", Color::Blue),
        EventKind::Any => ("❓", "ANY", Color::DarkGrey),
        EventKind::Other => ("❓", "OTHER", Color::DarkGrey),
    };

    println!("{} {}", emoji, format!("[{event_kind}]").with(color).bold());
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

        // Show diff if verbose mode is enabled and file was modified
        if verbose && matches!(event.kind, EventKind::Modify(_)) {
            show_file_diff(path, file_cache, parse_tx.clone());
        }
    }
}

/// Show diff for modified files
fn show_file_diff(
    path: &Path,
    file_cache: &Arc<Mutex<HashMap<PathBuf, String>>>,
    parse_tx: tokio::sync::mpsc::UnboundedSender<PathBuf>,
) {
    // Check file extension
    let extension = path.extension().and_then(|e| e.to_str()).unwrap_or("");

    // Only show diff for JSON/JSONL files
    if !matches!(extension, "json" | "jsonl") {
        if extension == "db" {
            println!(
                "    {} {}",
                "⚠️".with(Color::Yellow),
                "Diff not supported for .db files".with(Color::DarkGrey)
            );
        }
        return;
    }

    // Read current file content
    let current_content = match fs::read_to_string(path) {
        Ok(content) => content,
        Err(e) => {
            eprintln!(
                "    {} {} {}",
                "⚠️".with(Color::Yellow),
                "Failed to read file:".with(Color::Yellow),
                e.to_string().with(Color::DarkGrey)
            );
            return;
        }
    };

    // Validate JSON format
    if extension == "json" {
        if serde_json::from_str::<serde_json::Value>(&current_content).is_err() {
            println!(
                "    {} {}",
                "⚠️".with(Color::Yellow),
                "Failed to parse as valid JSON, skipping diff".with(Color::Yellow)
            );
            return;
        }
    } else if extension == "jsonl" {
        // For JSONL, check if at least one line is valid JSON
        let has_valid_json = current_content.lines().any(|line| {
            !line.trim().is_empty() && serde_json::from_str::<serde_json::Value>(line).is_ok()
        });

        if !has_valid_json {
            println!(
                "    {} {}",
                "⚠️".with(Color::Yellow),
                "Failed to parse as valid JSONL, skipping diff".with(Color::Yellow)
            );
            return;
        }
    }

    // Get previous content from cache
    let mut cache = file_cache.lock().unwrap();
    let previous_content = cache.get(path);

    if let Some(old_content) = previous_content {
        // Compute and display diff
        print_diff(old_content, &current_content);
    } else {
        println!(
            "    {} {}",
            "ℹ️".with(Color::Blue),
            "First time seeing this file, caching content...".with(Color::DarkGrey)
        );
    }

    // Update cache with current content
    cache.insert(path.to_path_buf(), current_content);
    drop(cache); // Release the mutex before async operation

    // Send path to parse channel for async parsing
    let _ = parse_tx.send(path.to_path_buf());
}

/// Parse file and log session information (async version)
async fn parse_and_log_sessions_async(path: &Path) {
    let parser_service = ParserService::new();
    let result = parser_service.parse_file(path).await;

    match result {
        Ok(_sessions) => {
            // ParserService already logs the sessions, so we don't need to do anything here
        }
        Err(e) => {
            eprintln!(
                "    {} {} {}",
                "⚠️".with(Color::Yellow),
                "Failed to parse file:".with(Color::Yellow),
                e.to_string().with(Color::DarkGrey)
            );
        }
    }
}

/// Print unified diff between two texts
fn print_diff(old: &str, new: &str) {
    let diff = TextDiff::from_lines(old, new);

    println!(
        "    {} {}",
        "📊".with(Color::Cyan),
        "Diff:".with(Color::Cyan).bold()
    );

    let mut has_changes = false;
    let mut line_count = 0;
    const MAX_DIFF_LINES: usize = 50; // Limit output to prevent spam

    for change in diff.iter_all_changes() {
        if line_count >= MAX_DIFF_LINES {
            println!(
                "    {} {}",
                "...".with(Color::DarkGrey),
                format!("(showing first {} lines)", MAX_DIFF_LINES).with(Color::DarkGrey)
            );
            break;
        }

        let (sign, color) = match change.tag() {
            ChangeTag::Delete => ("-", Color::Red),
            ChangeTag::Insert => ("+", Color::Green),
            ChangeTag::Equal => continue, // Skip unchanged lines for brevity
        };

        has_changes = true;
        line_count += 1;

        // Trim the line for display (remove trailing newline)
        let line = change.as_str().unwrap_or("").trim_end();

        // Truncate long lines
        let display_line = if line.len() > 120 {
            format!("{}...", &line[..120])
        } else {
            line.to_string()
        };

        println!("      {} {}", sign.with(color), display_line.with(color));
    }

    if !has_changes {
        println!(
            "    {} {}",
            "ℹ️".with(Color::Blue),
            "No content changes detected".with(Color::DarkGrey)
        );
    }
}

/// Detect provider from file path using directory and file patterns
pub fn detect_provider(file_path: &Path) -> ProviderDetection {
    let file_name = file_path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    let parent_dir = file_path.parent().and_then(|p| p.to_str()).unwrap_or("");

    // Get provider configs from global registry
    let registry = ProviderRegistry::global();
    let providers = registry.all_configs_with_names();

    // Priority 1: Check directory + file pattern match (most specific)
    for (config, provider_name) in &providers {
        if let Some(matched_pattern) = find_matching_pattern(&config.file_patterns, file_name) {
            // Check if directory also matches
            let dir_matches = check_directory_match(config, parent_dir);

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
        if check_directory_match(config, parent_dir) {
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
            if parts.len() == 2 && file_name.starts_with(parts[0]) && file_name.ends_with(parts[1])
            {
                return Some(pattern.clone());
            }
        } else if file_name == pattern {
            // Exact match for non-wildcard patterns (e.g., "store.db" only matches "store.db", not "store.db-wal")
            return Some(pattern.clone());
        }
    }
    None
}
