use anyhow::Result;
use console::{style, Emoji};
use indicatif::{ProgressBar, ProgressStyle};
use inquire::MultiSelect;
use std::sync::Arc;

use crate::database::{config, DatabaseManager};
use crate::services::{AutoDetectService, DetectedProvider, ImportService};

static ROCKET: Emoji<'_, '_> = Emoji("🚀 ", "");
static SPARKLES: Emoji<'_, '_> = Emoji("✨ ", "");
static CHECK: Emoji<'_, '_> = Emoji("✓ ", "[OK]");
static CROSS: Emoji<'_, '_> = Emoji("✗ ", "[X]");
static MAGNIFYING_GLASS: Emoji<'_, '_> = Emoji("🔍 ", "");

/// Run the interactive setup wizard for first-time users
pub async fn run_setup_wizard() -> Result<()> {
    println!(
        "\n{} {}",
        SPARKLES,
        style("Welcome to RetroChat!").bold().cyan()
    );
    println!();

    // Step 1: Ensure database is initialized
    ensure_database_initialized().await?;

    // Step 2: Auto-detect providers
    println!(
        "{} {}",
        MAGNIFYING_GLASS,
        style("Scanning for LLM chat histories...").bold()
    );
    let detected = AutoDetectService::scan_all();

    display_detected_providers(&detected);

    let valid_providers = AutoDetectService::valid_providers(&detected);

    if valid_providers.is_empty() {
        println!("\n{} {}", CROSS, style("No chat histories found.").yellow());
        println!();
        println!("💡 Quick setup:");
        println!("  1. Make sure you have chat files in default locations:");
        println!("     • Claude Code: ~/.claude/projects");
        println!("     • Cursor: ~/.cursor/chats");
        println!("     • Gemini: ~/.gemini/tmp");
        println!();
        println!(
            "  2. Or use: {} to import from a custom path",
            style("retrochat import --path /your/path").cyan()
        );
        println!();
        return Ok(());
    }

    // Step 3: Ask user which providers to import
    let selected = select_providers_to_import(&valid_providers)?;

    if selected.is_empty() {
        println!(
            "\n{} {}",
            style("ℹ").blue(),
            style("No providers selected. Skipping import.").dim()
        );
        println!(
            "  You can import later with: {}",
            style("retrochat import").cyan()
        );
        println!();
        return Ok(());
    }

    // Step 4: Import selected providers
    import_selected_providers(selected).await?;

    // Step 5: Show next steps
    println!();
    println!("{} {}", SPARKLES, style("All set!").bold().green());
    println!();
    println!("Next steps:");
    println!(
        "  • {} - Launch the TUI to explore your chats",
        style("retrochat").cyan()
    );
    println!(
        "  • {} - View usage statistics",
        style("retrochat stats").cyan()
    );
    println!(
        "  • {} - Search your messages",
        style("retrochat search \"keyword\"").cyan()
    );
    println!();

    Ok(())
}

/// Ensure database is initialized
async fn ensure_database_initialized() -> Result<()> {
    config::ensure_config_dir()?;
    let db_path = config::get_default_db_path()?;

    if db_path.exists() {
        println!("{} Database already initialized", CHECK);
        return Ok(());
    }

    println!("  Creating database at: {}", style(db_path.display()).dim());
    let _db_manager = DatabaseManager::new(&db_path).await?;
    println!("{} Database initialized", CHECK);

    Ok(())
}

/// Display detected providers in a nice format
fn display_detected_providers(detected: &[DetectedProvider]) {
    println!();
    for provider in detected {
        let status_icon = if provider.is_valid { CHECK } else { CROSS };
        let status_text = if provider.is_valid {
            style(format!("{} sessions", provider.estimated_sessions)).green()
        } else {
            style("Not found".to_string()).dim()
        };

        println!(
            "  {} {} - {}",
            status_icon,
            style(&provider.provider.to_string()).bold(),
            status_text
        );

        if provider.is_valid {
            for path in &provider.paths {
                println!("     {}", style(path.display()).dim());
            }
        }
    }
    println!();
}

/// Ask user to select which providers to import
fn select_providers_to_import(
    valid_providers: &[DetectedProvider],
) -> Result<Vec<DetectedProvider>> {
    let options: Vec<String> = valid_providers
        .iter()
        .map(|p| format!("{} ({} sessions)", p.provider, p.estimated_sessions))
        .collect();

    let selected_items = MultiSelect::new(
        "Select providers to import (Space to toggle, Enter to confirm):",
        options,
    )
    .with_default(&vec![0, 1, 2, 3][..valid_providers.len().min(4)])
    .prompt()?;

    // Find indices by matching selected items with original options
    let selected: Vec<DetectedProvider> = selected_items
        .iter()
        .filter_map(|selected_str| {
            valid_providers
                .iter()
                .find(|p| {
                    format!("{} ({} sessions)", p.provider, p.estimated_sessions) == *selected_str
                })
                .cloned()
        })
        .collect();

    Ok(selected)
}

/// Import selected providers with progress feedback
async fn import_selected_providers(selected: Vec<DetectedProvider>) -> Result<()> {
    println!();
    println!("{} Starting import...", ROCKET);
    println!();

    let db_path = config::get_default_db_path()?;
    let db_manager = Arc::new(DatabaseManager::new(&db_path).await?);
    let import_service = ImportService::new(db_manager);

    let total_sessions: usize = selected.iter().map(|p| p.estimated_sessions).sum();

    let pb = ProgressBar::new(total_sessions as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{bar:40.cyan/blue}] {pos}/{len} sessions ({percent}%)")?
            .progress_chars("━━╸"),
    );

    let mut total_imported_sessions = 0;
    let mut total_imported_messages = 0;

    for detected in selected {
        pb.set_message(format!("Importing {}...", detected.provider));

        for path in detected.paths {
            let batch_request = crate::services::BatchImportRequest {
                directory_path: path.to_string_lossy().to_string(),
                providers: None,
                project_name: None,
                overwrite_existing: Some(false),
                recursive: Some(true),
            };

            match import_service.import_batch(batch_request).await {
                Ok(response) => {
                    total_imported_sessions += response.total_sessions_imported;
                    total_imported_messages += response.total_messages_imported;
                    pb.inc(response.total_sessions_imported as u64);
                }
                Err(e) => {
                    pb.println(format!(
                        "{} Failed to import {}: {}",
                        CROSS,
                        style(detected.provider.to_string()).red(),
                        style(e).dim()
                    ));
                }
            }
        }
    }

    pb.finish_with_message("Import complete!");

    println!();
    println!("{} Import summary:", CHECK);
    println!(
        "  • {} sessions imported",
        style(total_imported_sessions).green().bold()
    );
    println!(
        "  • {} messages processed",
        style(total_imported_messages).green().bold()
    );

    Ok(())
}

/// Check if this is a first-time user (no database exists)
pub fn is_first_time_user() -> bool {
    let db_path = config::get_default_db_path().ok();
    db_path.is_none_or(|p| !p.exists())
}

/// Quick check if user needs setup
pub fn needs_setup() -> Result<bool> {
    // Check if database exists
    let db_path = config::get_default_db_path()?;
    if !db_path.exists() {
        return Ok(true);
    }

    // TODO: Could also check if database is empty (no sessions)
    // For now, just check if DB exists
    Ok(false)
}
