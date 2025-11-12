pub mod analytics;
pub mod config;
pub mod help;
pub mod import;
pub mod init;
pub mod query;
pub mod setup;
pub mod tui;
pub mod watch;

use clap::{Parser, Subcommand};
use std::sync::Arc;
use tokio::runtime::Runtime;

use crate::models::Provider;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Synchronize chat history from providers
    ///
    /// Available providers: all, claude, gemini, codex
    ///
    /// Examples:
    ///   retrochat sync claude gemini          # Import from multiple providers
    ///   retrochat sync all                    # Import from all providers
    ///   retrochat sync claude -w --verbose    # Watch mode with detailed output
    ///   retrochat sync --path ~/.claude/projects
    Sync {
        /// One or more providers to sync
        ///
        /// Available: all, claude, gemini, codex
        #[arg(value_enum)]
        providers: Vec<Provider>,

        /// A specific file or directory path to sync from
        #[arg(short, long)]
        path: Option<String>,

        /// Overwrite existing sessions if they already exist
        #[arg(short, long)]
        overwrite: bool,

        /// Watch for file changes and auto-import
        #[arg(short, long)]
        watch: bool,

        /// Show detailed diff of changes (applies to watch mode)
        #[arg(short = 'v', long)]
        verbose: bool,
    },

    /// List sessions with optional filters
    List {
        /// Filter by provider
        #[arg(long)]
        provider: Option<String>,
        /// Filter by project
        #[arg(long)]
        project: Option<String>,
        /// Page number (default: 1)
        #[arg(short, long)]
        page: Option<i32>,
        /// Page size (default: 20)
        #[arg(short = 's', long)]
        page_size: Option<i32>,
    },

    /// Show detailed information about a session
    Show {
        /// Session ID to view
        session_id: String,
    },

    /// Search messages by content
    Search {
        /// Search query
        query: String,
        /// Maximum number of results (default: 20)
        #[arg(short, long)]
        limit: Option<i32>,
        /// Messages since this time (e.g., "7 days ago", "2024-10-01", "yesterday")
        #[arg(long)]
        since: Option<String>,
        /// Messages until this time (e.g., "now", "2024-10-31", "today")
        #[arg(long)]
        until: Option<String>,
    },

    /// AI-powered session analysis
    Analysis {
        #[command(subcommand)]
        command: AnalysisCommands,
    },

    /// Export chat history
    Export {
        /// Output format: compact (default) or jsonl
        #[arg(long, short = 'f', default_value = "compact")]
        format: String,
        /// Messages since this time (e.g., "7 days ago", "2024-10-01", "yesterday")
        #[arg(long)]
        since: Option<String>,
        /// Messages until this time
        #[arg(long)]
        until: Option<String>,
        /// Filter by provider
        #[arg(long)]
        provider: Option<String>,
        /// Filter by role (User, Assistant, System)
        #[arg(long)]
        role: Option<String>,
        /// Maximum number of messages
        #[arg(long, short = 'n')]
        limit: Option<i32>,
        /// Reverse chronological order (newest first)
        #[arg(long, short = 'r')]
        reverse: bool,
        /// Disable message truncation in compact format (show full content)
        #[arg(long)]
        no_truncate: bool,
        /// Number of characters to show from the beginning (default: 400)
        #[arg(long, default_value = "400")]
        truncate_head: usize,
        /// Number of characters to show from the end (default: 200)
        #[arg(long, default_value = "200")]
        truncate_tail: usize,
        /// Output file path (optional, prints to stdout if not specified)
        #[arg(short, long)]
        output: Option<String>,
        /// Exclude tool use and tool result messages
        #[arg(long)]
        no_tool: bool,
    },

    /// Interactive setup wizard for first-time users
    Setup,

    /// Manage configuration settings
    Config {
        #[command(subcommand)]
        command: ConfigCommands,
    },
}

#[derive(Subcommand)]
pub enum AnalysisCommands {
    /// Run AI analysis on a session
    Run {
        /// Session ID to analyze (if not provided, will prompt for selection)
        session_id: Option<String>,
        /// Custom prompt for analysis
        #[arg(long)]
        custom_prompt: Option<String>,
        /// Analyze all sessions
        #[arg(long)]
        all: bool,
        /// Process in background (simplified - just shows progress)
        #[arg(long)]
        background: bool,
        /// Output format: enhanced (default), markdown, json, or plain
        #[arg(long, short = 'f', default_value = "enhanced")]
        format: String,
        /// Use plain text format (alias for --format=plain)
        #[arg(long)]
        plain: bool,
    },

    /// Show analysis results
    Show {
        /// Session ID to show results for
        session_id: Option<String>,
        /// Show all results
        #[arg(long)]
        all: bool,
        /// Output format
        #[arg(short, long, default_value = "text")]
        format: String,
    },

    /// Show analysis request status
    Status {
        /// Show all active operations
        #[arg(long)]
        all: bool,
        /// Watch for status changes
        #[arg(long)]
        watch: bool,
        /// Show history of completed operations
        #[arg(long)]
        history: bool,
    },

    /// Cancel analysis request
    Cancel {
        /// Request ID to cancel (if not provided, will list active requests)
        request_id: Option<String>,
        /// Cancel all active requests
        #[arg(long)]
        all: bool,
    },
}

#[derive(Subcommand)]
pub enum ConfigCommands {
    /// Get a configuration value
    Get {
        /// Configuration key to get
        key: String,
    },
    /// Set a configuration value
    Set {
        /// Configuration key to set
        key: String,
        /// Value to set
        value: String,
    },
    /// Remove a configuration value
    Unset {
        /// Configuration key to remove
        key: String,
    },
    /// List all configuration values
    List,
    /// Show the path to the config file
    Path,
}

impl Cli {
    pub fn run(self) -> anyhow::Result<()> {
        let rt = Runtime::new()?;
        let rt_arc = Arc::new(rt);

        // Create cleanup handler for analysis commands
        let _cleanup_guard = if matches!(self.command, Some(Commands::Analysis { .. })) {
            Some(self.create_analytics_request_cleanup_handler(rt_arc.clone())?)
        } else {
            None
        };

        rt_arc.block_on(async {
            // Handle no subcommand - default behavior
            let command = match self.command {
                None => {
                    // Check if first-time user (no database exists)
                    if setup::is_first_time_user() {
                        // Run setup wizard (interactive import)
                        if let Err(e) = setup::run_setup_wizard().await {
                            eprintln!("Setup failed: {e}");
                            return Err(e);
                        }
                    }

                    // After setup (or if DB already exists), launch TUI
                    println!("{}",console::style("────────────────────────────────────────────────────────────────────────────").dim());
                    println!("  {} {}", console::style("🚀").bold(), console::style("Launching TUI").bold().cyan());
                    println!("{}", console::style("────────────────────────────────────────────────────────────────────────────").dim());
                    println!();

                    return tui::handle_tui_command().await;
                }
                Some(cmd) => cmd,
            };

            match command {
                // ═══════════════════════════════════════════════════
                // Data Synchronization
                // ═══════════════════════════════════════════════════
                Commands::Sync {
                    providers,
                    path,
                    overwrite,
                    watch,
                    verbose,
                } => {
                    if watch {
                        watch::handle_watch_command(path, providers, verbose, false).await
                    } else {
                        import::handle_import_command(path, providers, overwrite).await
                    }
                }

                // ═══════════════════════════════════════════════════
                // Session Management
                // ═══════════════════════════════════════════════════
                Commands::List {
                    provider,
                    project,
                    page,
                    page_size,
                } => query::handle_sessions_command(page, page_size, provider, project).await,

                Commands::Show { session_id } => {
                    query::handle_session_detail_command(session_id).await
                }

                Commands::Search {
                    query,
                    limit,
                    since,
                    until,
                } => query::handle_search_command(query, limit, since, until).await,

                // ═══════════════════════════════════════════════════
                // AI Analysis
                // ═══════════════════════════════════════════════════
                Commands::Analysis { command } => match command {
                    AnalysisCommands::Run {
                        session_id,
                        custom_prompt,
                        all,
                        background,
                        format,
                        plain,
                    } => {
                        analytics::handle_execute_command(
                            session_id,
                            custom_prompt,
                            all,
                            background,
                            format,
                            plain,
                        )
                        .await
                    }

                    AnalysisCommands::Show {
                        session_id,
                        all,
                        format,
                    } => analytics::handle_show_command(session_id, all, format).await,

                    AnalysisCommands::Status {
                        all,
                        watch,
                        history,
                    } => analytics::handle_status_command(all, watch, history).await,

                    AnalysisCommands::Cancel { request_id, all } => {
                        analytics::handle_cancel_command(request_id, all).await
                    }
                },

                // ═══════════════════════════════════════════════════
                // Export
                // ═══════════════════════════════════════════════════
                Commands::Export {
                    format,
                    since,
                    until,
                    provider,
                    role,
                    limit,
                    reverse,
                    no_truncate,
                    truncate_head,
                    truncate_tail,
                    output,
                    no_tool,
                } => {
                    // TODO: Handle output file if specified
                    if output.is_some() {
                        eprintln!(
                            "Warning: --output option is not yet implemented. Printing to stdout."
                        );
                    }

                    query::handle_timeline_command(query::TimelineParams {
                        since,
                        until,
                        provider,
                        role,
                        format,
                        limit,
                        reverse,
                        no_truncate,
                        truncate_head,
                        truncate_tail,
                        no_tool,
                    })
                    .await
                }

                // ═══════════════════════════════════════════════════
                // Setup & Configuration
                // ═══════════════════════════════════════════════════
                Commands::Setup => setup::run_setup_wizard().await,

                Commands::Config { command } => match command {
                    ConfigCommands::Get { key } => config::handle_config_get(key).await,
                    ConfigCommands::Set { key, value } => {
                        config::handle_config_set(key, value).await
                    }
                    ConfigCommands::Unset { key } => config::handle_config_unset(key).await,
                    ConfigCommands::List => config::handle_config_list().await,
                    ConfigCommands::Path => config::handle_config_path().await,
                },
            }
        })
    }

    fn create_analytics_request_cleanup_handler(
        &self,
        rt: Arc<Runtime>,
    ) -> anyhow::Result<crate::services::AnalyticsRequestCleanupHandler> {
        use crate::database::DatabaseManager;
        use crate::services::{
            google_ai::{GoogleAiClient, GoogleAiConfig},
            AnalyticsRequestCleanupHandler, AnalyticsRequestService,
        };

        // Create the necessary components synchronously
        let db_path = crate::database::config::get_default_db_path()?;
        let db_manager = rt.block_on(async { DatabaseManager::new(&db_path).await })?;

        // Get API key with priority: environment variable > config file
        let api_key = crate::config::get_google_ai_api_key()?.unwrap_or_default();

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

        Ok(AnalyticsRequestCleanupHandler::new(service, rt))
    }
}
