pub mod analytics;
pub mod import;
pub mod init;
pub mod query;
pub mod retrospect;
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
    /// Retrospection analysis using LLM
    Retrospect {
        #[command(subcommand)]
        command: RetrospectCommands,
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
pub enum RetrospectCommands {
    /// Analyze a specific session using LLM
    Analyze {
        /// Session ID to analyze
        session_id: String,
        /// Template ID to use for analysis (optional)
        #[arg(short, long)]
        template: Option<String>,
        /// Force re-analysis even if already exists
        #[arg(short, long)]
        force: bool,
    },
    /// List all retrospection analyses
    List {
        /// Session ID filter (optional)
        #[arg(short, long)]
        session: Option<String>,
        /// Template ID filter (optional)
        #[arg(short, long)]
        template: Option<String>,
        /// Page number (default: 1)
        #[arg(short, long)]
        page: Option<i32>,
        /// Page size (default: 20)
        #[arg(long)]
        page_size: Option<i32>,
    },
    /// Show detailed retrospection analysis
    Show {
        /// Analysis ID to show
        analysis_id: String,
    },
    /// Manage prompt templates
    Template {
        #[command(subcommand)]
        command: TemplateCommands,
    },
    /// Process pending analysis requests
    Process {
        /// Maximum number of requests to process (default: all)
        #[arg(short, long)]
        limit: Option<i32>,
        /// Force processing even if recently processed
        #[arg(short, long)]
        force: bool,
    },
}

#[derive(Subcommand)]
pub enum TemplateCommands {
    /// List all available templates
    List,
    /// Show template details
    Show {
        /// Template ID to show
        template_id: String,
    },
    /// Create new template
    Create {
        /// Template ID
        id: String,
        /// Template name
        name: String,
        /// Template description
        description: String,
        /// Template content (use @file.txt to read from file)
        content: String,
    },
    /// Update existing template
    Update {
        /// Template ID to update
        id: String,
        /// New template name (optional)
        #[arg(short, long)]
        name: Option<String>,
        /// New template description (optional)
        #[arg(short, long)]
        description: Option<String>,
        /// New template content (optional, use @file.txt to read from file)
        #[arg(short, long)]
        content: Option<String>,
    },
    /// Delete template
    Delete {
        /// Template ID to delete
        id: String,
        /// Force deletion without confirmation
        #[arg(short, long)]
        force: bool,
    },
    /// Import templates from file
    Import {
        /// Path to TOML file containing templates
        file: String,
        /// Overwrite existing templates
        #[arg(short, long)]
        overwrite: bool,
    },
    /// Export templates to file
    Export {
        /// Output file path
        file: String,
        /// Template IDs to export (optional, exports all if not specified)
        #[arg(short, long)]
        templates: Option<Vec<String>>,
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
                Commands::Retrospect { command } => match command {
                    RetrospectCommands::Analyze {
                        session_id,
                        template,
                        force,
                    } => retrospect::handle_analyze_command(session_id, template, force).await,
                    RetrospectCommands::List {
                        session,
                        template,
                        page,
                        page_size,
                    } => retrospect::handle_list_command(session, template, page, page_size).await,
                    RetrospectCommands::Show { analysis_id } => {
                        retrospect::handle_show_command(analysis_id).await
                    }
                    RetrospectCommands::Template { command } => match command {
                        TemplateCommands::List => retrospect::handle_template_list_command().await,
                        TemplateCommands::Show { template_id } => {
                            retrospect::handle_template_show_command(template_id).await
                        }
                        TemplateCommands::Create {
                            id,
                            name,
                            description,
                            content,
                        } => {
                            retrospect::handle_template_create_command(
                                id,
                                name,
                                description,
                                content,
                            )
                            .await
                        }
                        TemplateCommands::Update {
                            id,
                            name,
                            description,
                            content,
                        } => {
                            retrospect::handle_template_update_command(
                                id,
                                name,
                                description,
                                content,
                            )
                            .await
                        }
                        TemplateCommands::Delete { id, force } => {
                            retrospect::handle_template_delete_command(id, force).await
                        }
                        TemplateCommands::Import { file, overwrite } => {
                            retrospect::handle_template_import_command(file, overwrite).await
                        }
                        TemplateCommands::Export { file, templates } => {
                            retrospect::handle_template_export_command(file, templates).await
                        }
                    },
                    RetrospectCommands::Process { limit, force } => {
                        retrospect::handle_process_command(limit, force).await
                    }
                },
            }
        })
    }
}
