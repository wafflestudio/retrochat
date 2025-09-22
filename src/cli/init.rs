use anyhow::{Context, Result};
use std::path::Path;

use crate::database::connection::DatabaseManager;

pub async fn handle_init_command() -> Result<()> {
    println!("Initializing RetroChat database...");

    let db_path = "retrochat.db";

    // Check if database already exists
    if Path::new(db_path).exists() {
        println!("✓ Database already exists at: {db_path}");
        println!("  Use 'retrochat tui' to launch the interface");
        return Ok(());
    }

    // Initialize database
    let _db_manager =
        DatabaseManager::new(db_path).with_context(|| "Failed to create database manager")?;

    println!("✓ Database initialized successfully at: {db_path}");
    println!();
    println!("Next steps:");
    println!("  1. Import your chat files:");
    println!("     retrochat import scan");
    println!("     retrochat import file <path>");
    println!("     retrochat import batch <directory>");
    println!();
    println!("  2. Launch the TUI interface:");
    println!("     retrochat tui");
    println!();
    println!("  3. Generate insights:");
    println!("     retrochat analyze insights");

    Ok(())
}
