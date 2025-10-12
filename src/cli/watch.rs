use anyhow::Result;
use crossterm::style::{Color, Stylize};

use crate::models::Provider;
use crate::services::{collect_provider_paths, watch_paths_for_changes};

pub async fn handle_watch_command(
    path: Option<String>,
    providers: Vec<Provider>,
    verbose: bool,
    import: bool,
) -> Result<()> {
    if import {
        println!(
            "{} {} {}",
            "ℹ️".with(Color::Blue),
            "Note:".with(Color::Blue).bold(),
            "Auto-import feature is not yet implemented".with(Color::DarkGrey)
        );
    }

    // Collect paths to watch
    let watch_paths = if let Some(path) = path {
        vec![path]
    } else if providers.is_empty() {
        return Err(anyhow::anyhow!(
            "Please specify either --path or one or more providers to watch"
        ));
    } else {
        collect_provider_paths(&providers)?
    };

    if watch_paths.is_empty() {
        return Err(anyhow::anyhow!(
            "No paths to watch. Please specify --path or valid providers."
        ));
    }

    // Start watching
    watch_paths_for_changes(watch_paths, verbose).await
}
