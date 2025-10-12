use anyhow::{Context, Result};
use std::path::PathBuf;

use crate::env::database as env_vars;

/// Get the default database path in the user's home directory
/// Can be overridden by RETROCHAT_DB environment variable
pub fn get_default_db_path() -> Result<PathBuf> {
    // Check if RETROCHAT_DB environment variable is set
    if let Ok(db_path) = std::env::var(env_vars::RETROCHAT_DB) {
        return Ok(PathBuf::from(db_path));
    }

    // Otherwise use default path
    let home_dir = dirs::home_dir().context("Could not find home directory")?;
    Ok(home_dir.join(".retrochat").join("retrochat.db"))
}

/// Get the retrochat configuration directory path
pub fn get_config_dir() -> Result<PathBuf> {
    let home_dir = dirs::home_dir().context("Could not find home directory")?;
    Ok(home_dir.join(".retrochat"))
}

/// Ensure the retrochat configuration directory exists
pub fn ensure_config_dir() -> Result<()> {
    let config_dir = get_config_dir()?;
    std::fs::create_dir_all(&config_dir).with_context(|| {
        format!(
            "Failed to create config directory: {}",
            config_dir.display()
        )
    })?;
    Ok(())
}
