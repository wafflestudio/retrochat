use clap::Parser;
use retrochat::cli::{Cli, Commands};
use retrochat::logging::LoggingConfig;
use std::path::PathBuf;

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // Configure logging based on command
    let logging_config = match &cli.command {
        Some(Commands::Tui) | None => {
            // For TUI: log to file only, no stdout
            let log_dir = dirs::data_local_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join("retrochat")
                .join("logs");
            std::fs::create_dir_all(&log_dir)?;

            let log_file = log_dir.join(format!(
                "retrochat-{}.log",
                chrono::Local::now().format("%Y%m%d")
            ));

            LoggingConfig::from_env()
                .with_stdout(false) // Critical: disable stdout for TUI
                .with_file(log_file)
        }
        Some(Commands::Query { .. }) => {
            // For Query commands: disable stdout to keep output clean
            LoggingConfig::from_env().with_stdout(false)
        }
        _ => {
            // For other CLI commands: stdout is safe
            LoggingConfig::from_env()
        }
    };

    retrochat::logging::init_logging(logging_config)?;
    cli.run()
}
