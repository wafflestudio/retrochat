use anyhow::{Context, Result};
use std::path::PathBuf;

/// Get the default database path in the user's home directory
pub fn get_default_db_path() -> Result<PathBuf> {
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
