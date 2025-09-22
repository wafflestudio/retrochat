pub mod analytics;
pub mod import;
pub mod init;
pub mod query;
pub mod tui;

use clap::{Parser, Subcommand};
use tokio::runtime::Runtime;

#[derive(Parser)]
#[command(name = "retrochat")]
#[command(about = "LLM Agent Chat History Retrospect Application")]
#[command(version = "0.1.0")]
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
    /// Import chat files
    Import {
        #[command(subcommand)]
        command: ImportCommands,
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
}

#[derive(Subcommand)]
pub enum ImportCommands {
    /// Scan for chat files in directory
    Scan {
        /// Directory to scan (defaults to current directory)
        directory: Option<String>,
    },
    /// Import specific file
    File {
        /// Path to the chat file to import
        path: String,
    },
    /// Import batch of files from directory
    Batch {
        /// Directory containing chat files to import
        directory: String,
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

        rt.block_on(async {
            match self.command {
                Commands::Init => init::handle_init_command().await,
                Commands::Tui => tui::handle_tui_command().await,
                Commands::Import { command } => match command {
                    ImportCommands::Scan { directory } => {
                        import::handle_scan_command(directory).await
                    }
                    ImportCommands::File { path } => import::handle_import_file_command(path).await,
                    ImportCommands::Batch { directory } => {
                        import::handle_import_batch_command(directory).await
                    }
                },
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
            }
        })
    }
}
