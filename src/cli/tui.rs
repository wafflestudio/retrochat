use anyhow::{Context, Result};
use std::path::Path;

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
        println!("RetroChat - LLM Agent Chat History Retrospect Application");
        println!("=========================================================");
        println!();
        println!("Welcome to RetroChat! This application helps you analyze and explore");
        println!("your chat history with various LLM providers.");
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
        println!("Getting Started:");
        println!("1. Import your chat files:");
        println!("   - Scan for files: retrochat import scan");
        println!("   - Import specific file: retrochat import file <path>");
        println!("   - Import directory: retrochat import batch <directory>");
        println!();
        println!("2. Supported formats:");
        println!("   - Claude Code (.jsonl files)");
        println!("   - Gemini/Bard (.json files)");
        println!();
        println!("3. Launch TUI: retrochat tui");
        println!();
        println!("4. Generate insights: retrochat analyze insights");
        println!();
    }
}

pub async fn handle_tui_command() -> Result<()> {
    // Check if database file exists, if not provide guidance
    let db_path = Path::new("retrochat.db");
    if !db_path.exists() {
        TuiLauncher::print_welcome_message();
        println!("No database found. Let's get you started!");
        println!();
        TuiLauncher::print_getting_started();
        return Ok(());
    }

    // Initialize database manager
    let db_manager = DatabaseManager::new("retrochat.db")
        .await
        .with_context(|| "Failed to initialize database")?;

    // Create and launch TUI
    let launcher = TuiLauncher::new(db_manager);

    TuiLauncher::print_welcome_message();
    TuiLauncher::print_keyboard_shortcuts();

    match launcher.launch().await {
        Ok(()) => {
            println!("Thank you for using RetroChat!");
            Ok(())
        }
        Err(e) => {
            println!("\n❌ Failed to launch TUI interface: {e}");
            println!();
            println!("This might be because:");
            println!("  • You're not running in an interactive terminal");
            println!("  • Your terminal is too small (need at least 80x24 characters)");
            println!("  • Terminal doesn't support the required features");
            println!();
            println!("Alternative options:");
            println!("  • Use 'retrochat query sessions' to list sessions");
            println!("  • Use 'retrochat query search <query>' to search messages");
            println!("  • Use 'retrochat analyze insights' to generate analytics");
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
