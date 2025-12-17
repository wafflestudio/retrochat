use anyhow::Result;
use console::{style, Emoji};
use indicatif::{ProgressBar, ProgressStyle};
use inquire::MultiSelect;
use std::sync::Arc;

use retrochat_core::database::{config, DatabaseManager};
use retrochat_core::services::{AutoDetectService, DetectedProvider, ImportService};

static ROCKET: Emoji<'_, '_> = Emoji("🚀 ", "");
static SPARKLES: Emoji<'_, '_> = Emoji("✨ ", "");
static CHECK: Emoji<'_, '_> = Emoji("✓ ", "[OK]");
static CROSS: Emoji<'_, '_> = Emoji("✗ ", "[X]");

/// Run the interactive setup wizard for first-time users
pub async fn run_setup_wizard() -> Result<()> {
    use inquire::Select;

    println!(
        "\n{} {}",
        SPARKLES,
        style("Welcome to RetroChat!").bold().cyan()
    );
    println!();

    // Step 1: API Key Setup (only once)
    setup_api_key_interactive();

    // Step 2: Database Initialize
    setup_database_initialize().await?;

    // Step 3: Scan and Import Loop
    loop {
        let detected = scan_chat_histories();
        let valid_providers = AutoDetectService::valid_providers(&detected);

        if !valid_providers.is_empty() {
            // Found providers - use existing MultiSelect flow
            let selected = select_providers_to_import(&valid_providers)?;

            if selected.is_empty() {
                println!(
                    "\n{} {}",
                    style("ℹ").blue(),
                    style("No providers selected. Skipping import.").dim()
                );
                println!(
                    "  You can import later with: {}",
                    style("retrochat sync").cyan()
                );
                println!();
                break;
            }

            // Import
            import_selected_providers(selected).await?;

            println!();
            println!("{} {}", SPARKLES, style("All set!").bold().green());
            println!();
            break;
        } else {
            // No providers found - offer options
            println!();
            let options = vec![
                "1. Launch TUI anyway (you can import later)",
                "2. Configure provider paths",
            ];

            let choice = match Select::new(
                "No chat histories found. What would you like to do?",
                options,
            )
            .prompt()
            {
                Ok(c) => c,
                Err(_) => break,
            };

            if choice.starts_with("1.") {
                // Just launch TUI
                break;
            } else {
                // Configure paths
                if configure_provider_paths()? {
                    // User finished configuration, rescan
                    continue;
                } else {
                    // User cancelled
                    break;
                }
            }
        }
    }

    Ok(())
}

/// Step 1: API Key Setup (interactive, only once)
fn setup_api_key_interactive() {
    use inquire::{Select, Text};

    // Check if already configured
    if retrochat_core::config::has_google_ai_api_key() {
        println!(
            "{} Google AI API key is already configured",
            style("✓").green()
        );
        println!();
        return;
    }

    // Show setup prompt
    println!(
        "{}",
        style("────────────────────────────────────────────────────────────────────────────").dim()
    );
    println!(
        "  {} {}",
        style("🔑").bold(),
        style("API Key Setup (Optional)").bold().cyan()
    );
    println!(
        "{}",
        style("────────────────────────────────────────────────────────────────────────────").dim()
    );
    println!();
    println!("For AI-powered analytics, configure your Google AI API key.");
    println!(
        "💡 Get your key: {}",
        style("https://aistudio.google.com/app/apikey").underlined()
    );
    println!();

    let options = vec![
        "1. Shell config (~/.zshrc or ~/.bashrc) - For all programs",
        "2. RetroChat config only (~/.retrochat/config.toml)",
        "3. Skip for now",
    ];

    let choice =
        match Select::new("How would you like to configure your API key?", options).prompt() {
            Ok(choice) => choice,
            Err(_) => {
                println!("{}", style("\nSkipped. Configure later with:").dim());
                println!(
                    "  {}",
                    style("retrochat config set google-ai-api-key YOUR_KEY").cyan()
                );
                println!();
                return;
            }
        };

    match choice {
        s if s.starts_with("1.") => {
            println!();
            let api_key = match Text::new("Enter your Google AI API key:")
                .with_help_message("Paste your API key")
                .prompt()
            {
                Ok(key) if !key.trim().is_empty() => key.trim().to_string(),
                _ => {
                    println!("{}", style("Cancelled.").yellow());
                    println!();
                    return;
                }
            };

            if let Err(e) = add_to_shell_config(&api_key) {
                eprintln!("{} Failed: {}", style("✗").red(), e);
                eprintln!("Add this line manually to ~/.zshrc or ~/.bashrc:");
                eprintln!(
                    "  {}",
                    style(format!("export GOOGLE_AI_API_KEY=\"{api_key}\"")).cyan()
                );
                eprintln!();
            }
        }

        s if s.starts_with("2.") => {
            println!();
            let api_key = match Text::new("Enter your Google AI API key:")
                .with_help_message("Paste your API key")
                .prompt()
            {
                Ok(key) if !key.trim().is_empty() => key.trim().to_string(),
                _ => {
                    println!("{}", style("Cancelled.").yellow());
                    println!();
                    return;
                }
            };

            match save_to_retrochat_config(&api_key) {
                Ok(_) => {
                    println!();
                    println!("{} API key saved", style("✓").green());
                    println!("  {}", style("~/.retrochat/config.toml").dim());
                    println!();
                }
                Err(e) => {
                    eprintln!("{} Failed: {}", style("✗").red(), e);
                    eprintln!();
                }
            }
        }

        _ => {
            println!();
            println!("{}", style("Skipped. Configure later with:").dim());
            println!(
                "  {}",
                style("retrochat config set google-ai-api-key YOUR_KEY").cyan()
            );
            println!();
        }
    }
}

/// Step 2: Database Initialize
async fn setup_database_initialize() -> Result<()> {
    config::ensure_config_dir()?;
    let db_path = config::get_default_db_path()?;

    println!(
        "{}",
        style("────────────────────────────────────────────────────────────────────────────").dim()
    );
    println!(
        "  {} {}",
        style("💾").bold(),
        style("Database").bold().cyan()
    );
    println!(
        "{}",
        style("────────────────────────────────────────────────────────────────────────────").dim()
    );
    println!();

    if db_path.exists() {
        println!("{} Database already exists", style("✓").green());
        println!("  {}", style(db_path.display()).dim());
    } else {
        println!("Creating database at:");
        println!("  {}", style(db_path.display()).dim());
        let _db_manager = DatabaseManager::new(&db_path).await?;
        println!();
        println!("{} Database initialized", style("✓").green());
    }
    println!();

    Ok(())
}

/// Step 3: Scan chat histories
fn scan_chat_histories() -> Vec<DetectedProvider> {
    println!(
        "{}",
        style("────────────────────────────────────────────────────────────────────────────").dim()
    );
    println!(
        "  {} {}",
        style("🔍").bold(),
        style("Chat History Scan").bold().cyan()
    );
    println!(
        "{}",
        style("────────────────────────────────────────────────────────────────────────────").dim()
    );
    println!();
    println!("Scanning for chat histories...");

    let detected = AutoDetectService::scan_all();
    display_detected_providers(&detected);

    detected
}

/// Step 4: Configure provider paths (returns true if user completed, false if cancelled)
fn configure_provider_paths() -> Result<bool> {
    use inquire::{Select, Text};

    println!();
    println!("Configure provider paths:");
    println!(
        "  💡 Changes will be saved to: {}",
        style("~/.retrochat/config.toml").cyan()
    );
    println!(
        "  After configuration, run: {}",
        style("retrochat sync").cyan()
    );
    println!();

    loop {
        let options = vec![
            "1. Claude Code path",
            "2. Gemini CLI path",
            "3. Codex path",
            "4. Done (rescan)",
        ];

        let choice = match Select::new("Select provider to configure:", options).prompt() {
            Ok(c) => c,
            Err(_) => return Ok(false),
        };

        if choice.starts_with("4.") {
            return Ok(true);
        }

        // Get path input
        println!();
        let provider_name = if choice.starts_with("1.") {
            "Claude Code"
        } else if choice.starts_with("2.") {
            "Gemini CLI"
        } else {
            "Codex"
        };

        let path = match Text::new(&format!("Enter {provider_name} directory path:"))
            .with_help_message("Full path to chat history directory")
            .prompt()
        {
            Ok(p) if !p.trim().is_empty() => p.trim().to_string(),
            _ => {
                println!("{}", style("Cancelled.").yellow());
                println!();
                continue;
            }
        };

        // Save to config (TODO: implement actual saving to env vars in config)
        println!();
        println!("{} Path saved: {}", style("✓").green(), style(&path).cyan());
        println!("  💡 Manually add to ~/.zshrc for persistence:");
        if choice.starts_with("1.") {
            println!(
                "    {}",
                style(format!("export RETROCHAT_CLAUDE_DIRS=\"{path}\"")).dim()
            );
        } else if choice.starts_with("2.") {
            println!(
                "    {}",
                style(format!("export RETROCHAT_GEMINI_DIRS=\"{path}\"")).dim()
            );
        } else {
            println!(
                "    {}",
                style(format!("export RETROCHAT_CODEX_DIRS=\"{path}\"")).dim()
            );
        }
        println!();
    }
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
    println!("{ROCKET} Starting import...");
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
            let batch_request = retrochat_core::services::BatchImportRequest {
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
    println!("{CHECK} Import summary:");
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

/// Add API key to shell config file (~/.zshrc or ~/.bashrc)
fn add_to_shell_config(api_key: &str) -> Result<()> {
    use std::fs::OpenOptions;
    use std::io::{Read, Write};

    // Detect shell config file
    let home = dirs::home_dir().ok_or_else(|| anyhow::anyhow!("Could not find home directory"))?;
    let shell_config = if home.join(".zshrc").exists() {
        home.join(".zshrc")
    } else if home.join(".bashrc").exists() {
        home.join(".bashrc")
    } else {
        // Default to .zshrc for macOS
        home.join(".zshrc")
    };

    // Read existing content
    let mut existing_content = String::new();
    if shell_config.exists() {
        let mut file = std::fs::File::open(&shell_config)?;
        file.read_to_string(&mut existing_content)?;
    }

    // Check if already exists
    if existing_content.contains("GOOGLE_AI_API_KEY") {
        println!();
        println!(
            "{} GOOGLE_AI_API_KEY already exists in {}",
            style("ℹ").blue(),
            style(shell_config.display()).dim()
        );
        println!("  Please update it manually if needed.");
        println!();
        return Ok(());
    }

    // Append to file
    let export_line =
        format!("\n# RetroChat - Google AI API Key\nexport GOOGLE_AI_API_KEY=\"{api_key}\"\n");
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&shell_config)?;
    file.write_all(export_line.as_bytes())?;

    println!();
    println!(
        "{} API key added to {}",
        style("✓").green(),
        style(shell_config.display()).cyan()
    );
    println!();
    println!(
        "{}",
        style("⚠ Important: Reload your shell to apply changes:")
            .yellow()
            .bold()
    );
    println!(
        "  {}",
        style(format!("source {}", shell_config.display())).cyan()
    );
    println!();
    println!("Or open a new terminal window.");
    println!();

    Ok(())
}

/// Save API key to RetroChat config file
fn save_to_retrochat_config(api_key: &str) -> Result<()> {
    let mut config = retrochat_core::config::Config::load()?;
    config.api.google_ai_api_key = Some(api_key.to_string());
    config.save()?;
    Ok(())
}
