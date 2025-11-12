use anyhow::{Context, Result};
use std::path::Path;

use crate::cli::help;
use crate::database::DatabaseManager;
use crate::tui::app::App;

pub struct TuiLauncher {
    db_manager: DatabaseManager,
}

impl TuiLauncher {
    pub fn new(db_manager: DatabaseManager) -> Self {
        Self { db_manager }
    }

    pub async fn launch(&self) -> Result<()> {
        println!("Initializing RetroChat TUI...");

        // Ensure database is initialized
        self.initialize_database().await?;

        // Launch actual TUI application
        println!("Launching TUI interface...");
        self.launch_tui().await?;

        Ok(())
    }

    async fn initialize_database(&self) -> Result<()> {
        // Database should already be initialized by DatabaseManager
        // This is just a verification step
        println!("✓ Database initialized");
        Ok(())
    }

    async fn launch_tui(&self) -> Result<()> {
        use crossterm::{
            event::{DisableMouseCapture, EnableMouseCapture},
            execute,
            terminal::{
                disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
            },
        };
        use ratatui::{backend::CrosstermBackend, Terminal};
        use std::io;

        // Check if we're in a proper terminal environment
        if !atty::is(atty::Stream::Stdout) {
            return Err(anyhow::anyhow!(
                "TUI requires an interactive terminal. Please run this command in a terminal."
            ));
        }

        // Check terminal size
        let (width, height) = crossterm::terminal::size()
            .map_err(|e| anyhow::anyhow!("Failed to get terminal size: {e}"))?;

        if width < 80 || height < 24 {
            return Err(anyhow::anyhow!(
                "Terminal too small. Please resize to at least 80x24 characters."
            ));
        }

        // Setup terminal with proper error handling
        enable_raw_mode().map_err(|e| anyhow::anyhow!("Failed to enable raw mode: {e}"))?;

        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)
            .map_err(|e| anyhow::anyhow!("Failed to setup terminal: {e}"))?;

        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)
            .map_err(|e| anyhow::anyhow!("Failed to create terminal: {e}"))?;

        // Create and run app
        let db_manager = std::sync::Arc::new(self.db_manager.clone());
        let mut app =
            App::new(db_manager).map_err(|e| anyhow::anyhow!("Failed to create app: {e}"))?;

        let result = app.run(&mut terminal).await;

        // Restore terminal with proper error handling
        let _ = disable_raw_mode();
        let _ = execute!(
            terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        );
        let _ = terminal.show_cursor();

        result.map_err(|e| anyhow::anyhow!("TUI runtime error: {e}"))
    }

    pub fn print_welcome_message() {
        println!("RetroChat - LLM Chat History Analysis");
        println!();
    }

    pub fn print_keyboard_shortcuts() {
        println!("Keyboard Shortcuts:");
        println!("  q, Ctrl+C     - Quit application");
        println!("  Tab           - Switch between panels");
        println!("  ↑/↓           - Navigate lists");
        println!("  Enter         - Select item/Enter detail view");
        println!("  Esc           - Go back/Exit detail view");
        println!("  /             - Search mode");
        println!("  r             - Refresh data");
        println!("  h, ?          - Show help");
        println!();
    }

    pub fn print_getting_started() {
        help::print_full_getting_started();
    }
}

/// Print API key setup guide if not configured
fn print_api_key_setup_guide() {
    use console::style;

    // Check if API key is already configured
    if crate::config::has_google_ai_api_key() {
        println!("{} Google AI API key is configured", style("✓").green());
        println!();
        return;
    }

    // API key is not configured, show setup guide
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
    println!("To use AI-powered analytics, configure your Google AI API key:");
    println!();
    println!(
        "{} {}",
        style("Method 1:").green().bold(),
        style("Environment Variable (All Programs) ✅").bold()
    );
    println!();
    println!("  Add to ~/.zshrc (macOS):");
    println!(
        "    {}",
        style("export GOOGLE_AI_API_KEY=\"your-api-key-here\"").cyan()
    );
    println!();
    println!("  Reload: {}", style("source ~/.zshrc").cyan());
    println!();
    println!(
        "{} {}",
        style("Method 2:").green().bold(),
        style("Config File (This Program Only) ⚙️").bold()
    );
    println!();
    println!(
        "  {}",
        style("retrochat config set google-ai-api-key YOUR_KEY").cyan()
    );
    println!();
    println!(
        "💡 Get your key: {}",
        style("https://aistudio.google.com/app/apikey").underlined()
    );
    println!();
    println!("Skip for now? No problem! Set it up later anytime.");
    println!();
}

pub async fn handle_tui_command() -> Result<()> {
    use console::style;

    // Check if database file exists, if not provide guidance
    let db_path = crate::database::config::get_default_db_path()?;
    if !db_path.exists() {
        println!(
            "{}",
            style("════════════════════════════════════════════════════════════════════════════")
                .dim()
        );
        println!(
            "                {}",
            style("RetroChat - LLM Chat History Analysis").bold().cyan()
        );
        println!(
            "{}",
            style("════════════════════════════════════════════════════════════════════════════")
                .dim()
        );
        println!();
        println!(
            "{} No database found. Let's set things up!",
            style("👋").bold()
        );
        println!();

        // Check if Google AI API key is configured
        print_api_key_setup_guide();

        TuiLauncher::print_getting_started();
        return Ok(());
    }

    // Initialize database manager
    let db_manager = DatabaseManager::new(&db_path)
        .await
        .with_context(|| "Failed to initialize database")?;

    // Create and launch TUI
    let launcher = TuiLauncher::new(db_manager);

    // Launch directly (welcome messages are shown by setup wizard)
    match launcher.launch().await {
        Ok(()) => Ok(()),
        Err(e) => {
            println!("\n❌ Failed to launch TUI interface: {e}");
            println!();
            println!("This might be because:");
            println!("  • You're not running in an interactive terminal");
            println!("  • Your terminal is too small (need at least 80x24 characters)");
            println!("  • Terminal doesn't support the required features");
            println!();
            println!("Alternative options:");
            println!("  • Use '$ retrochat query sessions' to list sessions");
            println!("  • Use '$ retrochat query search <query>' to search messages");
            println!("  • Use '$ retrochat analytics insights' to generate analytics");
            println!();
            println!(
                "If you believe this is a bug, please check your terminal setup and try again."
            );
            Err(e)
        }
    }
}

pub async fn initialize_database_if_needed(database_path: &str) -> Result<DatabaseManager> {
    let db_path = Path::new(database_path);
    let is_new_database = !db_path.exists();

    let db_manager = DatabaseManager::new(database_path)
        .await
        .with_context(|| format!("Failed to initialize database: {database_path}"))?;

    if is_new_database {
        println!("✓ Created new database: {database_path}");
    } else {
        println!("✓ Connected to existing database: {database_path}");
    }

    Ok(db_manager)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[tokio::test]
    async fn test_initialize_database() {
        let temp_db = NamedTempFile::new().unwrap();
        let db_path = temp_db.path().to_string_lossy().to_string();

        let result = initialize_database_if_needed(&db_path).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_tui_launcher_creation() {
        let db_manager = DatabaseManager::new(":memory:").await.unwrap();
        let _launcher = TuiLauncher::new(db_manager);
        // Just test that we can create the launcher
    }
}
