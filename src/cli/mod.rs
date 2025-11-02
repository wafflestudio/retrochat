pub mod analytics;
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

use crate::env::apis as env_vars;
use crate::models::Provider;
use analytics::AnalyticsCommands;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Initialize application database
    Init,
    /// Launch TUI interface
    Tui,
    /// Import chat files from a path or from one or more providers
    ///
    /// Available providers: all, claude, gemini, codex, cursor
    ///
    /// Examples:
    ///   retrochat import claude cursor        # Import from multiple providers
    ///   retrochat import all                  # Import from all providers
    ///   retrochat import --path ~/.claude/projects
    Import {
        /// A specific file or directory path to import from
        #[arg(short, long)]
        path: Option<String>,

        /// One or more providers to import from
        ///
        /// Available: all, claude, gemini, codex, cursor
        #[arg(value_enum)]
        providers: Vec<Provider>,

        /// Overwrite existing sessions if they already exist
        #[arg(short, long)]
        overwrite: bool,
    },
    /// Watch files for changes and show diffs
    ///
    /// Available providers: all, claude, gemini, codex, cursor
    ///
    /// Examples:
    ///   retrochat watch all --verbose         # Watch all providers with detailed output
    ///   retrochat watch claude cursor         # Watch specific providers
    ///   retrochat watch --path ~/.claude/projects --verbose
    Watch {
        /// A specific file or directory path to watch
        #[arg(short, long)]
        path: Option<String>,

        /// One or more providers to watch
        ///
        /// Available: all, claude, gemini, codex, cursor
        #[arg(value_enum)]
        providers: Vec<Provider>,

        /// Show detailed diff of changes
        #[arg(short = 'v', long)]
        verbose: bool,

        /// Automatically import changes when detected (future feature)
        #[arg(short, long)]
        import: bool,
    },
    /// Analytics sessions with request tracking
    Analytics {
        #[command(subcommand)]
        command: AnalyticsCommands,
    },
    /// Query sessions and search messages
    Query {
        #[command(subcommand)]
        command: QueryCommands,
    },
    /// Interactive setup wizard for first-time users
    Setup,
    /// [Alias for 'import'] Add chat files interactively or from providers
    ///
    /// This is a more intuitive alias for the import command.
    /// Examples:
    ///   retrochat add                 # Interactive mode
    ///   retrochat add --path /path    # Import from path
    Add {
        /// A specific file or directory path to import from
        #[arg(short, long)]
        path: Option<String>,

        /// One or more providers to import from
        #[arg(value_enum)]
        providers: Vec<Provider>,

        /// Overwrite existing sessions if they already exist
        #[arg(short, long)]
        overwrite: bool,
    },
    /// [Alias for 'query search'] Search messages by content
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
    /// [Alias for 'analytics execute'] Review and analytics a chat session
    Review {
        /// Session ID to review (optional, will prompt if not provided)
        session_id: Option<String>,
    },
}

#[derive(Subcommand)]
pub enum QueryCommands {
    /// List sessions with optional filters
    Sessions {
        /// Page number (default: 1)
        #[arg(short, long)]
        page: Option<i32>,
        /// Page size (default: 20)
        #[arg(short = 's', long)]
        page_size: Option<i32>,
        /// Filter by provider
        #[arg(long)]
        provider: Option<String>,
        /// Filter by project
        #[arg(long)]
        project: Option<String>,
    },
    /// Show detailed information about a specific session
    Session {
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
    /// Query messages by time range
    Timeline {
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
        /// Output format: compact (default) or jsonl
        #[arg(long, short = 'F', default_value = "compact")]
        format: String,
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
    },
}

impl Cli {
    pub fn run(self) -> anyhow::Result<()> {
        let rt = Runtime::new()?;
        let rt_arc = Arc::new(rt);

        // Create cleanup handler for analytics commands
        let _cleanup_guard = if matches!(self.command, Some(Commands::Analytics { .. })) {
            Some(self.create_analytics_request_cleanup_handler(rt_arc.clone())?)
        } else {
            None
        };

        rt_arc.block_on(async {
            // Handle no subcommand - default behavior
            let command = match self.command {
                None => {
                    // Check if first-time user
                    if setup::is_first_time_user() {
                        // Run setup wizard
                        return setup::run_setup_wizard().await;
                    } else {
                        // Launch TUI by default
                        return tui::handle_tui_command().await;
                    }
                }
                Some(cmd) => cmd,
            };

            match command {
                Commands::Init => init::handle_init_command().await,
                Commands::Tui => tui::handle_tui_command().await,
                Commands::Import {
                    path,
                    providers,
                    overwrite,
                } => import::handle_import_command(path, providers, overwrite).await,
                Commands::Watch {
                    path,
                    providers,
                    verbose,
                    import,
                } => watch::handle_watch_command(path, providers, verbose, import).await,
                Commands::Analytics { command } => match command {
                    AnalyticsCommands::Execute {
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
                    AnalyticsCommands::Show {
                        session_id,
                        all,
                        format,
                    } => analytics::handle_show_command(session_id, all, format).await,
                    AnalyticsCommands::Status {
                        all,
                        watch,
                        history,
                    } => analytics::handle_status_command(all, watch, history).await,
                    AnalyticsCommands::Cancel { request_id, all } => {
                        analytics::handle_cancel_command(request_id, all).await
                    }
                },
                Commands::Query { command } => match command {
                    QueryCommands::Sessions {
                        page,
                        page_size,
                        provider,
                        project,
                    } => query::handle_sessions_command(page, page_size, provider, project).await,
                    QueryCommands::Session { session_id } => {
                        query::handle_session_detail_command(session_id).await
                    }
                    QueryCommands::Search {
                        query,
                        limit,
                        since,
                        until,
                    } => query::handle_search_command(query, limit, since, until).await,
                    QueryCommands::Timeline {
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
                    } => {
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
                        })
                        .await
                    }
                },
                // New commands
                Commands::Setup => setup::run_setup_wizard().await,
                Commands::Add {
                    path,
                    providers,
                    overwrite,
                } => {
                    // If no arguments, run interactive setup
                    if path.is_none() && providers.is_empty() {
                        setup::run_setup_wizard().await
                    } else {
                        import::handle_import_command(path, providers, overwrite).await
                    }
                }
                Commands::Search {
                    query,
                    limit,
                    since,
                    until,
                } => query::handle_search_command(query, limit, since, until).await,
                Commands::Review { session_id } => {
                    // Delegate to analytics execute
                    // TODO: Could make this more interactive
                    if let Some(sid) = session_id {
                        analytics::handle_execute_command(
                            Some(sid),
                            None,
                            false,
                            false,
                            "enhanced".to_string(),
                            false,
                        )
                        .await
                    } else {
                        Err(anyhow::anyhow!(
                            "Session ID required. Use: retrochat review <SESSION_ID>"
                        ))
                    }
                }
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

        let api_key = std::env::var(env_vars::GOOGLE_AI_API_KEY).unwrap_or_else(|_| "".to_string()); // Use empty string if not set, as default() does

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
