use clap::Parser;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::runtime::Runtime;

mod commands;
use commands::{Cli, Commands};

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // Configure logging based on command
    let logging_config = match &cli.command {
        None => {
            // For TUI (default): log to file only, no stdout
            // Use same directory as DB (~/.retrochat/logs)
            let config_dir = retrochat_core::database::config::get_config_dir()
                .unwrap_or_else(|_| PathBuf::from("."));
            let log_dir = config_dir.join("logs");
            std::fs::create_dir_all(&log_dir)?;

            let log_file = log_dir.join(format!(
                "retrochat-{}.log",
                chrono::Local::now().format("%Y%m%d")
            ));

            retrochat_core::logging::LoggingConfig::from_env()
                .with_stdout(false) // Critical: disable stdout for TUI
                .with_file(log_file)
        }
        Some(Commands::List { .. })
        | Some(Commands::Show { .. })
        | Some(Commands::Search { .. })
        | Some(Commands::Export { .. }) => {
            // For query/output commands: disable stdout to keep output clean
            retrochat_core::logging::LoggingConfig::from_env().with_stdout(false)
        }
        _ => {
            // For other CLI commands: stdout is safe
            retrochat_core::logging::LoggingConfig::from_env()
        }
    };

    retrochat_core::logging::init_logging(logging_config)?;

    // Run CLI or TUI
    let rt = Runtime::new()?;
    let rt_arc = Arc::new(rt);

    // Create cleanup handler for analysis commands
    let _cleanup_guard = if matches!(cli.command, Some(Commands::Analysis { .. })) {
        Some(create_analytics_request_cleanup_handler(&rt_arc)?)
    } else {
        None
    };

    rt_arc.block_on(async {
        match cli.command {
            None => {
                // No subcommand → Check for first-time setup, then launch TUI
                if commands::setup::is_first_time_user() {
                    // Run setup wizard (interactive import)
                    if let Err(e) = commands::setup::run_setup_wizard().await {
                        eprintln!("Setup failed: {e}");
                        return Err(e);
                    }
                }

                // After setup (or if DB already exists), launch TUI
                println!(
                    "{}",
                    console::style(
                        "────────────────────────────────────────────────────────────────────────────"
                    )
                    .dim()
                );
                println!(
                    "  {} {}",
                    console::style("🚀").bold(),
                    console::style("Launching TUI").bold().cyan()
                );
                println!(
                    "{}",
                    console::style(
                        "────────────────────────────────────────────────────────────────────────────"
                    )
                    .dim()
                );
                println!();

                retrochat_tui::run_tui().await
            }
            Some(cmd) => {
                // Has subcommand → Run CLI command
                commands::run_command(cmd).await
            }
        }
    })
}

fn create_analytics_request_cleanup_handler(
    rt: &Arc<Runtime>,
) -> anyhow::Result<retrochat_core::services::AnalyticsRequestCleanupHandler> {
    use retrochat_core::database::DatabaseManager;
    use retrochat_core::services::{
        google_ai::{GoogleAiClient, GoogleAiConfig},
        AnalyticsRequestCleanupHandler, AnalyticsRequestService,
    };

    // Create the necessary components synchronously
    let db_path = retrochat_core::database::config::get_default_db_path()?;
    let db_manager = rt.block_on(async { DatabaseManager::new(&db_path).await })?;

    // Get API key with priority: environment variable > config file
    let api_key = retrochat_core::config::get_google_ai_api_key()?.unwrap_or_default();

    let google_ai_config = if api_key.is_empty() {
        GoogleAiConfig::default()
    } else {
        GoogleAiConfig::new(api_key)
    };
    let google_ai_client = GoogleAiClient::new(google_ai_config)?;
    let service = Arc::new(AnalyticsRequestService::new(
        Arc::new(db_manager),
        google_ai_client,
    ));

    Ok(AnalyticsRequestCleanupHandler::new(service, rt.clone()))
}
