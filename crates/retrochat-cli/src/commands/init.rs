use anyhow::{Context, Result};

use crate::commands::help;
use retrochat_core::database::{config, DatabaseManager};

pub async fn handle_init_command() -> Result<()> {
    // Ensure config directory exists
    config::ensure_config_dir()?;

    let db_path = config::get_default_db_path()?;

    // Check if database already exists
    if db_path.exists() {
        println!("✓ Database already exists at: {}", db_path.display());
        help::print_getting_started();
        return Ok(());
    }

    println!("Initializing RetroChat database...");
    println!("  Creating database at: {}", db_path.display());

    // Initialize database
    let _db_manager = DatabaseManager::new(&db_path)
        .await
        .with_context(|| "Failed to create database manager")?;

    println!(
        "✓ Database initialized successfully at: {}",
        db_path.display()
    );
    help::print_getting_started();

    Ok(())
}
