use anyhow::{Context, Result};
use crossterm::style::{Color, Stylize};
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::{Path, PathBuf};

use crate::models::provider::config::{
    ClaudeCodeConfig, CodexConfig, CursorAgentConfig, GeminiCliConfig,
};
use crate::models::Provider;

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
pub async fn watch_paths_for_changes(paths: Vec<String>) -> Result<()> {
    use std::sync::mpsc::channel;

    println!(
        "{}",
        "👁️  Starting file watcher...".with(Color::Cyan).bold()
    );
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

    // Process events
    loop {
        match rx.recv() {
            Ok(event) => {
                print_event(&event);
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
    }
}

/// Detect provider from file path using directory and file patterns
pub fn detect_provider(file_path: &Path) -> ProviderDetection {
    let file_name = file_path.file_name().and_then(|n| n.to_str()).unwrap_or("");

    let parent_dir = file_path.parent().and_then(|p| p.to_str()).unwrap_or("");

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
