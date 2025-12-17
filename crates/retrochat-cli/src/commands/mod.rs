pub mod analytics;
pub mod config;
pub mod help;
pub mod import;
pub mod init;
pub mod query;
pub mod setup;
pub mod watch;

use clap::{Parser, Subcommand};
use retrochat_core::models::Provider;

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

    /// Export a session transcript to JSON file
    ExportSession {
        /// Session ID to export
        session_id: String,
        /// Output file path (prints to stdout if not specified)
        #[arg(short, long)]
        output: Option<String>,
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
    },

    /// Show analysis results
    Show {
        /// Session ID to show results for
        session_id: Option<String>,
        /// Show all results
        #[arg(long)]
        all: bool,
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

/// Route and execute CLI commands
pub async fn run_command(command: Commands) -> anyhow::Result<()> {
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
                self::watch::handle_watch_command(path, providers, verbose, false).await
            } else {
                self::import::handle_import_command(path, providers, overwrite).await
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
        } => self::query::handle_sessions_command(page, page_size, provider, project).await,

        Commands::Show { session_id } => {
            self::query::handle_session_detail_command(session_id).await
        }

        Commands::ExportSession { session_id, output } => {
            self::query::handle_export_session_command(session_id, output).await
        }

        Commands::Search {
            query,
            limit,
            since,
            until,
        } => self::query::handle_search_command(query, limit, since, until).await,

        // ═══════════════════════════════════════════════════
        // AI Analysis
        // ═══════════════════════════════════════════════════
        Commands::Analysis { command } => match command {
            AnalysisCommands::Run {
                session_id,
                custom_prompt,
                all,
                background,
            } => {
                self::analytics::handle_execute_command(session_id, custom_prompt, all, background)
                    .await
            }

            AnalysisCommands::Show { session_id, all } => {
                self::analytics::handle_show_command(session_id, all).await
            }

            AnalysisCommands::Status {
                all,
                watch,
                history,
            } => self::analytics::handle_status_command(all, watch, history).await,

            AnalysisCommands::Cancel { request_id, all } => {
                self::analytics::handle_cancel_command(request_id, all).await
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
                eprintln!("Warning: --output option is not yet implemented. Printing to stdout.");
            }

            self::query::handle_timeline_command(self::query::TimelineParams {
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
        Commands::Setup => self::setup::run_setup_wizard().await,

        Commands::Config { command } => match command {
            ConfigCommands::Get { key } => self::config::handle_config_get(key).await,
            ConfigCommands::Set { key, value } => self::config::handle_config_set(key, value).await,
            ConfigCommands::Unset { key } => self::config::handle_config_unset(key).await,
            ConfigCommands::List => self::config::handle_config_list().await,
            ConfigCommands::Path => self::config::handle_config_path().await,
        },
    }
}
