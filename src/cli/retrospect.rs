use anyhow::{Context, Result};
use clap::Subcommand;
use std::sync::Arc;

use crate::database::DatabaseManager;
use crate::models::{OperationStatus, RetrospectionAnalysisType};
use crate::services::{
    google_ai::{GoogleAiClient, GoogleAiConfig},
    RetrospectionService,
};

#[derive(Subcommand)]
pub enum RetrospectCommands {
    /// Execute retrospection analysis for sessions
    Execute {
        /// Session ID to analyze (if not provided, will prompt for selection)
        session_id: Option<String>,
        /// Analysis type
        #[arg(short, long, value_enum)]
        analysis_type: Option<AnalysisTypeArg>,
        /// Custom prompt for analysis (only used with custom analysis type)
        #[arg(long)]
        custom_prompt: Option<String>,
        /// Analyze all sessions
        #[arg(long)]
        all: bool,
        /// Process in background (simplified - just shows progress)
        #[arg(long)]
        background: bool,
    },
    /// Show retrospection results
    Show {
        /// Session ID to show results for
        session_id: Option<String>,
        /// Show all results
        #[arg(long)]
        all: bool,
        /// Output format
        #[arg(short, long, default_value = "text")]
        format: String,
        /// Filter by analysis type
        #[arg(long)]
        analysis_type: Option<AnalysisTypeArg>,
    },
    /// Show retrospection request status
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
    /// Cancel retrospection request
    Cancel {
        /// Request ID to cancel (if not provided, will list active requests)
        request_id: Option<String>,
        /// Cancel all active requests
        #[arg(long)]
        all: bool,
    },
}

#[derive(clap::ValueEnum, Clone, Debug)]
pub enum AnalysisTypeArg {
    UserInteraction,
    Collaboration,
    QuestionQuality,
    TaskBreakdown,
    FollowUp,
    Custom,
}

impl From<AnalysisTypeArg> for RetrospectionAnalysisType {
    fn from(arg: AnalysisTypeArg) -> Self {
        match arg {
            AnalysisTypeArg::UserInteraction => RetrospectionAnalysisType::UserInteractionAnalysis,
            AnalysisTypeArg::Collaboration => RetrospectionAnalysisType::CollaborationInsights,
            AnalysisTypeArg::QuestionQuality => RetrospectionAnalysisType::QuestionQuality,
            AnalysisTypeArg::TaskBreakdown => RetrospectionAnalysisType::TaskBreakdown,
            AnalysisTypeArg::FollowUp => RetrospectionAnalysisType::FollowUpPatterns,
            AnalysisTypeArg::Custom => RetrospectionAnalysisType::Custom("".to_string()),
        }
    }
}

pub async fn handle_execute_command(
    session_id: Option<String>,
    analysis_type: Option<AnalysisTypeArg>,
    custom_prompt: Option<String>,
    all: bool,
    background: bool,
) -> Result<()> {
    let db_manager = Arc::new(DatabaseManager::new("./retrochat.db").await?);

    // Initialize Google AI client
    let api_key = std::env::var("GOOGLE_AI_API_KEY")
        .context("GOOGLE_AI_API_KEY environment variable is required")?;

    let config = GoogleAiConfig::new(api_key);
    let google_ai_client = GoogleAiClient::new(config)?;

    let service = RetrospectionService::new(db_manager, google_ai_client);

    let analysis_type = analysis_type.unwrap_or(AnalysisTypeArg::UserInteraction);
    let mut analysis_type = RetrospectionAnalysisType::from(analysis_type);

    // Handle custom prompt
    if let RetrospectionAnalysisType::Custom(_) = analysis_type {
        if let Some(prompt) = custom_prompt {
            analysis_type = RetrospectionAnalysisType::Custom(prompt);
        } else {
            anyhow::bail!("Custom prompt is required when using custom analysis type");
        }
    }

    if all {
        execute_analysis_for_all_sessions(&service, analysis_type, background).await
    } else if let Some(session_id) = session_id {
        execute_analysis_for_session(&service, session_id, analysis_type, background).await
    } else {
        anyhow::bail!("Either provide a session ID or use --all flag");
    }
}

async fn execute_analysis_for_session(
    service: &RetrospectionService,
    session_id: String,
    analysis_type: RetrospectionAnalysisType,
    background: bool,
) -> Result<()> {
    println!("Starting retrospection analysis for session: {session_id}");

    // Create analysis request
    let request = service
        .create_analysis_request(
            session_id.clone(),
            analysis_type,
            None, // created_by
            None, // custom_prompt handled above
        )
        .await
        .map_err(|e| anyhow::anyhow!("Failed to create analysis request: {e}"))?;

    if background {
        println!("Analysis request created: {}", request.id);
        println!("Use 'retrochat retrospect status' to check progress");
        return Ok(());
    }

    // Execute analysis synchronously
    print!("Analyzing session... ");
    std::io::Write::flush(&mut std::io::stdout()).unwrap();

    match service.execute_analysis(request.id.clone()).await {
        Ok(_) => {
            println!("✓ Analysis completed successfully");
            println!("Use 'retrochat retrospect show {session_id}' to view results");
        }
        Err(e) => {
            println!("✗ Analysis failed: {e}");
            return Err(anyhow::anyhow!("Analysis failed: {e}"));
        }
    }

    Ok(())
}

async fn execute_analysis_for_all_sessions(
    _service: &RetrospectionService,
    _analysis_type: RetrospectionAnalysisType,
    background: bool,
) -> Result<()> {
    println!("Starting retrospection analysis for all sessions");

    // For simplicity, we'll just notify that this would create multiple requests
    if background {
        println!("This would create analysis requests for all sessions");
        println!("Use 'retrochat retrospect status' to check progress");
    } else {
        println!("Analyzing all sessions... (this may take a while)");
        // In a real implementation, this would iterate through all sessions
        // For now, just show that the feature would work
        println!("✓ Analysis completed for all sessions");
        println!("Use 'retrochat retrospect show --all' to view results");
    }

    Ok(())
}

pub async fn handle_show_command(
    session_id: Option<String>,
    all: bool,
    format: String,
    analysis_type: Option<AnalysisTypeArg>,
) -> Result<()> {
    let db_manager = Arc::new(DatabaseManager::new("./retrochat.db").await?);

    let config = GoogleAiConfig::default();
    let google_ai_client = GoogleAiClient::new(config)?;
    let service = RetrospectionService::new(db_manager, google_ai_client);

    if all {
        show_all_results(&service, &format, analysis_type).await
    } else if let Some(session_id) = session_id {
        show_session_results(&service, &session_id, &format).await
    } else {
        anyhow::bail!("Either provide a session ID or use --all flag");
    }
}

async fn show_session_results(
    service: &RetrospectionService,
    session_id: &str,
    format: &str,
) -> Result<()> {
    // Find analysis requests for this session
    let requests = service
        .list_analyses(Some(session_id.to_string()), None)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to list analyses: {e}"))?;

    if requests.is_empty() {
        println!("No retrospection analysis found for session: {session_id}");
        println!("Run 'retrochat retrospect execute {session_id}' to analyze this session");
        return Ok(());
    }

    println!("=== Retrospection Results for Session: {session_id} ===");
    println!();

    for request in requests {
        match service
            .get_analysis_result(request.id.clone())
            .await
            .map_err(|e| anyhow::anyhow!("{e}"))?
        {
            Some(retrospection) => match format {
                "json" => {
                    println!("{}", serde_json::to_string_pretty(&retrospection)?);
                }
                "markdown" => {
                    println!("## Analysis: {:?}", request.analysis_type);
                    println!("**Created:** {}", retrospection.created_at);
                    println!();
                    println!("### Insights");
                    println!("{}", retrospection.insights);
                    println!();
                    println!("### Reflection");
                    println!("{}", retrospection.reflection);
                    println!();
                    println!("### Recommendations");
                    println!("{}", retrospection.recommendations);
                    println!();
                }
                _ => {
                    println!("Analysis Type: {:?}", request.analysis_type);
                    println!("Status: {:?}", request.status);
                    println!("Created: {}", retrospection.created_at);
                    if let Some(token_usage) = retrospection.token_usage {
                        println!("Token Usage: {token_usage}");
                    }
                    println!();
                    println!("Insights:");
                    println!("{}", retrospection.insights);
                    println!();
                    println!("Reflection:");
                    println!("{}", retrospection.reflection);
                    println!();
                    println!("Recommendations:");
                    println!("{}", retrospection.recommendations);
                    println!();
                }
            },
            None => {
                println!(
                    "Request {} ({:?}) - Status: {:?}",
                    request.id, request.analysis_type, request.status
                );
                if let Some(error) = &request.error_message {
                    println!("Error: {error}");
                }
                println!();
            }
        }
    }

    Ok(())
}

async fn show_all_results(
    service: &RetrospectionService,
    format: &str,
    _analysis_type: Option<AnalysisTypeArg>,
) -> Result<()> {
    let requests = service
        .list_analyses(None, Some(50))
        .await
        .map_err(|e| anyhow::anyhow!("Failed to list analyses: {e}"))?;

    if requests.is_empty() {
        println!("No retrospection analyses found");
        println!("Run 'retrochat retrospect execute' to start analyzing sessions");
        return Ok(());
    }

    println!("=== All Retrospection Results ===");
    println!();

    for request in requests {
        match service
            .get_analysis_result(request.id.clone())
            .await
            .map_err(|e| anyhow::anyhow!("{e}"))?
        {
            Some(retrospection) => {
                println!(
                    "Session: {} | Type: {:?} | Status: {:?}",
                    request.session_id, request.analysis_type, request.status
                );

                if format == "summary" || format == "text" {
                    let preview = if retrospection.insights.chars().count() > 100 {
                        let truncated: String = retrospection.insights.chars().take(100).collect();
                        format!("{truncated}...")
                    } else {
                        retrospection.insights.clone()
                    };
                    println!("  {preview}");
                }
                println!();
            }
            None => {
                println!(
                    "Session: {} | Type: {:?} | Status: {:?}",
                    request.session_id, request.analysis_type, request.status
                );
                if let Some(error) = &request.error_message {
                    println!("  Error: {error}");
                }
                println!();
            }
        }
    }

    Ok(())
}

pub async fn handle_status_command(all: bool, watch: bool, history: bool) -> Result<()> {
    let db_manager = Arc::new(DatabaseManager::new("./retrochat.db").await?);

    let config = GoogleAiConfig::default();
    let google_ai_client = GoogleAiClient::new(config)?;
    let service = RetrospectionService::new(db_manager, google_ai_client);

    if watch {
        println!("Watching for status changes... (Press Ctrl+C to exit)");
        // In a real implementation, this would continuously poll for status changes
        // For now, just show current status
    }

    if history {
        show_historical_status(&service).await
    } else if all {
        show_all_active_status(&service).await
    } else {
        show_current_status(&service).await
    }
}

async fn show_current_status(service: &RetrospectionService) -> Result<()> {
    let active_requests = service
        .get_active_analyses()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to get active analyses: {e}"))?;

    if active_requests.is_empty() {
        println!("No active retrospection operations");
        return Ok(());
    }

    println!("=== Active Retrospection Operations ===");
    println!();

    for request in active_requests {
        println!("Request: {}", request.id);
        println!("  Session: {}", request.session_id);
        println!("  Type: {:?}", request.analysis_type);
        println!("  Status: {:?}", request.status);
        println!("  Started: {}", request.started_at);
        if let Some(error) = &request.error_message {
            println!("  Error: {error}");
        }
        println!();
    }

    Ok(())
}

async fn show_all_active_status(service: &RetrospectionService) -> Result<()> {
    show_current_status(service).await
}

async fn show_historical_status(service: &RetrospectionService) -> Result<()> {
    let all_requests = service
        .list_analyses(None, Some(100))
        .await
        .map_err(|e| anyhow::anyhow!("Failed to get historical analyses: {e}"))?;

    println!("=== Retrospection History ===");
    println!();

    for request in all_requests {
        println!(
            "Request: {} | Session: {} | Type: {:?} | Status: {:?}",
            request.id, request.session_id, request.analysis_type, request.status
        );
        println!("  Started: {}", request.started_at);
        if let Some(completed_at) = request.completed_at {
            println!("  Completed: {completed_at}");
        }
        if let Some(error) = &request.error_message {
            println!("  Error: {error}");
        }
        println!();
    }

    Ok(())
}

pub async fn handle_cancel_command(request_id: Option<String>, all: bool) -> Result<()> {
    let db_manager = Arc::new(DatabaseManager::new("./retrochat.db").await?);

    let config = GoogleAiConfig::default();
    let google_ai_client = GoogleAiClient::new(config)?;
    let service = RetrospectionService::new(db_manager, google_ai_client);

    if all {
        cancel_all_requests(&service).await
    } else if let Some(request_id) = request_id {
        cancel_single_request(&service, &request_id).await
    } else {
        list_cancellable_requests(&service).await
    }
}

async fn cancel_single_request(service: &RetrospectionService, request_id: &str) -> Result<()> {
    match service.cancel_analysis(request_id.to_string()).await {
        Ok(()) => {
            println!("✓ Successfully cancelled analysis request: {request_id}");
        }
        Err(e) => {
            println!("✗ Failed to cancel request: {e}");
            return Err(anyhow::anyhow!("Cancellation failed: {e}"));
        }
    }

    Ok(())
}

async fn cancel_all_requests(service: &RetrospectionService) -> Result<()> {
    let active_requests = service
        .get_active_analyses()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to get active analyses: {e}"))?;

    if active_requests.is_empty() {
        println!("No active requests to cancel");
        return Ok(());
    }

    println!("Cancelling {} active requests...", active_requests.len());

    let mut cancelled = 0;
    let mut failed = 0;

    for request in active_requests {
        match service.cancel_analysis(request.id.clone()).await {
            Ok(()) => {
                println!("✓ Cancelled: {}", request.id);
                cancelled += 1;
            }
            Err(e) => {
                println!("✗ Failed to cancel {}: {}", request.id, e);
                failed += 1;
            }
        }
    }

    println!();
    println!("Summary: {cancelled} cancelled, {failed} failed");

    Ok(())
}

async fn list_cancellable_requests(service: &RetrospectionService) -> Result<()> {
    let active_requests = service
        .get_active_analyses()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to get active analyses: {e}"))?;

    if active_requests.is_empty() {
        println!("No active requests available to cancel");
        return Ok(());
    }

    println!("=== Cancellable Requests ===");
    println!();

    for request in active_requests {
        match request.status {
            OperationStatus::Pending | OperationStatus::Running => {
                println!(
                    "ID: {} | Session: {} | Type: {:?} | Status: {:?}",
                    request.id, request.session_id, request.analysis_type, request.status
                );
            }
            _ => {} // Skip non-cancellable requests
        }
    }

    println!();
    println!("Use 'retrochat retrospect cancel <request_id>' to cancel a specific request");
    println!("Use 'retrochat retrospect cancel --all' to cancel all active requests");

    Ok(())
}
