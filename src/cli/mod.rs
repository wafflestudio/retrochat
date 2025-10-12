pub mod analytics;
pub mod help;
pub mod import;
pub mod init;
pub mod query;
pub mod retrospect;
pub mod tui;

use clap::{Parser, Subcommand};
use std::sync::Arc;
use tokio::runtime::Runtime;

use crate::models::Provider;
use retrospect::RetrospectCommands;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
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
    /// Analyze usage data
    Analyze {
        #[command(subcommand)]
        command: AnalyzeCommands,
    },
    /// Query sessions and search messages
    Query {
        #[command(subcommand)]
        command: QueryCommands,
    },
    /// Retrospection analysis for chat sessions
    Retrospect {
        #[command(subcommand)]
        command: RetrospectCommands,
    },
}

#[derive(Subcommand)]
pub enum AnalyzeCommands {
    /// Generate usage insights
    Insights,
    /// Export analytics data
    Export {
        /// Export format (json, csv, txt)
        format: String,
        /// Output file path (optional)
        #[arg(short, long)]
        output: Option<String>,
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
    },
}

impl Cli {
    pub fn run(self) -> anyhow::Result<()> {
        let rt = Runtime::new()?;
        let rt_arc = Arc::new(rt);

        // Create cleanup handler for retrospection commands
        let _cleanup_guard = if matches!(self.command, Commands::Retrospect { .. }) {
            Some(self.create_retrospection_cleanup_handler(rt_arc.clone())?)
        } else {
            None
        };

        rt_arc.block_on(async {
            match self.command {
                Commands::Init => init::handle_init_command().await,
                Commands::Tui => tui::handle_tui_command().await,
                Commands::Import {
                    path,
                    providers,
                    overwrite,
                } => import::handle_import_command(path, providers, overwrite).await,
                Commands::Analyze { command } => match command {
                    AnalyzeCommands::Insights => analytics::handle_insights_command().await,
                    AnalyzeCommands::Export { format, output } => {
                        analytics::handle_export_command(format, output).await
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
                    QueryCommands::Search { query, limit } => {
                        query::handle_search_command(query, limit).await
                    }
                },
                Commands::Retrospect { command } => match command {
                    RetrospectCommands::Execute {
                        session_id,
                        analysis_type,
                        custom_prompt,
                        all,
                        background,
                    } => {
                        retrospect::handle_execute_command(
                            session_id,
                            analysis_type,
                            custom_prompt,
                            all,
                            background,
                        )
                        .await
                    }
                    RetrospectCommands::Show {
                        session_id,
                        all,
                        format,
                        analysis_type,
                    } => {
                        retrospect::handle_show_command(session_id, all, format, analysis_type)
                            .await
                    }
                    RetrospectCommands::Status {
                        all,
                        watch,
                        history,
                    } => retrospect::handle_status_command(all, watch, history).await,
                    RetrospectCommands::Cancel { request_id, all } => {
                        retrospect::handle_cancel_command(request_id, all).await
                    }
                },
            }
        })
    }

    fn create_retrospection_cleanup_handler(
        &self,
        rt: Arc<Runtime>,
    ) -> anyhow::Result<crate::services::RetrospectionCleanupHandler> {
        use crate::database::DatabaseManager;
        use crate::services::{
            google_ai::{GoogleAiClient, GoogleAiConfig},
            RetrospectionCleanupHandler, RetrospectionService,
        };

        // Create the necessary components synchronously
        let db_path = crate::database::config::get_default_db_path()?;
        let db_manager = rt.block_on(async { DatabaseManager::new(&db_path).await })?;

        let api_key = std::env::var("GOOGLE_AI_API_KEY").unwrap_or_else(|_| "".to_string()); // Use empty string if not set, as default() does

        let google_ai_config = if api_key.is_empty() {
            GoogleAiConfig::default()
        } else {
            GoogleAiConfig::new(api_key)
        };
        let google_ai_client = GoogleAiClient::new(google_ai_config)?;
        let service = Arc::new(RetrospectionService::new(
            Arc::new(db_manager),
            google_ai_client,
        ));

        Ok(RetrospectionCleanupHandler::new(service, rt))
    }
}
