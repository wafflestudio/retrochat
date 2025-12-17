use anyhow::Result;
use console::style;

use retrochat_core::config::Config;

/// Handle config get command
pub async fn handle_config_get(key: String) -> Result<()> {
    let config = Config::load()?;

    if let Some(value) = config.get(&key) {
        println!("{value}");
        Ok(())
    } else {
        anyhow::bail!("Config key '{key}' not found");
    }
}

/// Handle config set command
pub async fn handle_config_set(key: String, value: String) -> Result<()> {
    let mut config = Config::load()?;

    config.set(&key, value.clone())?;
    config.save()?;

    println!(
        "{} Config '{}' set successfully",
        style("✓").green(),
        style(&key).cyan()
    );
    println!(
        "  Saved to: {}",
        style(Config::get_config_path()?.display()).dim()
    );

    Ok(())
}

/// Handle config unset command
pub async fn handle_config_unset(key: String) -> Result<()> {
    let mut config = Config::load()?;

    config.unset(&key)?;
    config.save()?;

    println!(
        "{} Config '{}' removed",
        style("✓").green(),
        style(&key).cyan()
    );

    Ok(())
}

/// Handle config list command
pub async fn handle_config_list() -> Result<()> {
    let config = Config::load()?;
    let items = config.list();

    if items.is_empty() {
        println!("{}", style("No configuration set.").dim());
        println!();
        println!("💡 Set a config value:");
        println!(
            "  {}",
            style("retrochat config set google-ai-api-key YOUR_KEY").cyan()
        );
    } else {
        println!("{}", style("Configuration:").bold());
        println!();
        for (key, value) in items {
            println!("  {} = {}", style(key).cyan(), style(value).dim());
        }
        println!();
        println!(
            "  Config file: {}",
            style(Config::get_config_path()?.display()).dim()
        );
    }

    Ok(())
}

/// Handle config path command
pub async fn handle_config_path() -> Result<()> {
    let config_path = Config::get_config_path()?;
    println!("{}", config_path.display());
    Ok(())
}
