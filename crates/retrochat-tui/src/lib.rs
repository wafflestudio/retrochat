pub mod app;
pub mod components;
pub mod events;
pub mod session_detail;
pub mod session_list;
pub mod state;
pub mod tool_display;
pub mod utils;

pub use app::{App, AppMode, AppState};
pub use session_detail::SessionDetailWidget;
pub use session_list::SessionListWidget;

use anyhow::{Context, Result};
use retrochat_core::database::DatabaseManager;

/// Main entry point for TUI mode
/// This function handles all the terminal setup, TUI execution, and teardown
pub async fn run_tui() -> Result<()> {
    use crossterm::{
        event::{DisableMouseCapture, EnableMouseCapture},
        execute,
        terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    };
    use ratatui::{backend::CrosstermBackend, Terminal};
    use std::io;

    // Get database path
    let db_path = retrochat_core::database::config::get_default_db_path()?;

    // Check if database exists
    if !db_path.exists() {
        return Err(anyhow::anyhow!(
            "Database not found. Please run 'retrochat setup' first or sync some chat history."
        ));
    }

    // Initialize database manager
    let db_manager = std::sync::Arc::new(
        DatabaseManager::new(&db_path)
            .await
            .with_context(|| "Failed to initialize database")?,
    );

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
    let mut terminal =
        Terminal::new(backend).map_err(|e| anyhow::anyhow!("Failed to create terminal: {e}"))?;

    // Create and run app
    let mut app = App::new(db_manager).map_err(|e| anyhow::anyhow!("Failed to create app: {e}"))?;

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
