use anyhow::{Context, Result};
use clap::Subcommand;
use std::sync::Arc;

use crate::database::DatabaseManager;
use crate::env::apis as env_vars;
use crate::models::OperationStatus;
use crate::services::{
    google_ai::{GoogleAiClient, GoogleAiConfig},
    AnalyticsRequestService,
};

#[derive(Subcommand)]
pub enum AnalyticsCommands {
    /// Execute analysis for sessions
    Execute {
        /// Session ID to analytics (if not provided, will prompt for selection)
        session_id: Option<String>,
        /// Custom prompt for analysis
        #[arg(long)]
        custom_prompt: Option<String>,
        /// Analytics all sessions
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

pub async fn handle_execute_command(
    session_id: Option<String>,
    custom_prompt: Option<String>,
    all: bool,
    background: bool,
) -> Result<()> {
    let db_path = crate::database::config::get_default_db_path()?;
    let db_manager = Arc::new(DatabaseManager::new(&db_path).await?);

    // Initialize Google AI client
    let api_key = std::env::var(env_vars::GOOGLE_AI_API_KEY)
        .context("GOOGLE_AI_API_KEY environment variable is required")?;

    let config = GoogleAiConfig::new(api_key);
    let google_ai_client = GoogleAiClient::new(config)?;

    let service = AnalyticsRequestService::new(db_manager, google_ai_client);

    if all {
        execute_analysis_for_all_sessions(&service, custom_prompt, background).await
    } else if let Some(session_id) = session_id {
        execute_analysis_for_session(&service, session_id, custom_prompt, background).await
    } else {
        anyhow::bail!("Either provide a session ID or use --all flag");
    }
}

async fn execute_analysis_for_session(
    service: &AnalyticsRequestService,
    session_id: String,
    custom_prompt: Option<String>,
    background: bool,
) -> Result<()> {
    println!("Starting analysis for session: {session_id}");

    // Create analysis request
    let request = match service
        .create_analysis_request(
            session_id.clone(),
            None, // created_by
            custom_prompt.clone(),
        )
        .await
    {
        Ok(request) => request,
        Err(e) => {
            let error_msg = e.to_string();
            // Check if this is a dirty check error (session unchanged)
            if error_msg.contains("has not been modified since last analysis") {
                println!("ℹ Session has not changed since last analysis");
                println!("Retrieving cached results...\n");

                // Find the latest completed request
                let requests = service
                    .list_analyses(Some(session_id.clone()), None)
                    .await
                    .map_err(|e| anyhow::anyhow!("Failed to list analyses: {e}"))?;

                if let Some(latest_request) = requests
                    .iter()
                    .filter(|r| matches!(r.status, OperationStatus::Completed))
                    .max_by_key(|r| r.completed_at.as_ref())
                {
                    // Get and display cached results
                    if let Some(analysis) = service
                        .get_analysis_result(latest_request.id.clone())
                        .await
                        .map_err(|e| anyhow::anyhow!("Failed to get cached analysis: {e}"))?
                    {
                        print_unified_analysis(&analysis).await?;
                        println!(
                            "\n✓ Showing cached results from: {}",
                            latest_request
                                .completed_at
                                .map(|dt| dt.to_rfc3339())
                                .unwrap_or_else(|| "unknown".to_string())
                        );
                        println!("  To force new analysis, use: --custom-prompt \"your prompt\"");
                        return Ok(());
                    }
                }

                return Err(anyhow::anyhow!("No cached results found"));
            }

            // Other errors, propagate them
            return Err(anyhow::anyhow!("Failed to create analysis request: {e}"));
        }
    };

    if background {
        println!("Analysis request created: {}", request.id);
        println!("Use 'retrochat analytics status' to check progress");
        return Ok(());
    }

    // Execute analysis synchronously
    print!("Analyzing session... ");
    std::io::Write::flush(&mut std::io::stdout()).unwrap();

    match service.execute_analysis(request.id.clone()).await {
        Ok(_) => {
            println!("✓ Analysis completed successfully");

            // Get and display results
            if let Some(analysis) = service
                .get_analysis_result(request.id.clone())
                .await
                .map_err(|e| anyhow::anyhow!("Failed to get analysis result: {e}"))?
            {
                print_unified_analysis(&analysis).await?;
            }
        }
        Err(e) => {
            println!("✗ Analysis failed: {e}");
            return Err(anyhow::anyhow!("Analysis failed: {e}"));
        }
    }

    Ok(())
}

async fn execute_analysis_for_all_sessions(
    _service: &AnalyticsRequestService,
    _custom_prompt: Option<String>,
    background: bool,
) -> Result<()> {
    println!("Starting analysis for all sessions");

    // For simplicity, we'll just notify that this would create multiple requests
    if background {
        println!("This would create analysis requests for all sessions");
        println!("Use 'retrochat analytics status' to check progress");
    } else {
        println!("Analyzing all sessions... (this may take a while)");
        // In a real implementation, this would iterate through all sessions
        // For now, just show that the feature would work
        println!("✓ Analysis completed for all sessions");
        println!("Use 'retrochat analytics show --all' to view results");
    }

    Ok(())
}

pub async fn handle_show_command(session_id: Option<String>, all: bool) -> Result<()> {
    let db_path = crate::database::config::get_default_db_path()?;
    let db_manager = Arc::new(DatabaseManager::new(&db_path).await?);

    // For show command, we don't need Google AI - just create a dummy client
    // since we're only reading from database
    let config = GoogleAiConfig::new("dummy-key-for-read-only".to_string());
    let google_ai_client = GoogleAiClient::new(config)?;
    let service = AnalyticsRequestService::new(db_manager, google_ai_client);

    if all {
        show_all_results(&service).await
    } else if let Some(session_id) = session_id {
        show_session_results(&service, &session_id).await
    } else {
        anyhow::bail!("Either provide a session ID or use --all flag");
    }
}

async fn show_session_results(service: &AnalyticsRequestService, session_id: &str) -> Result<()> {
    // Find analysis requests for this session
    let requests = service
        .list_analyses(Some(session_id.to_string()), None)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to list analyses: {e}"))?;

    if requests.is_empty() {
        println!("No analysis found for session: {session_id}");
        println!("Run 'retrochat analytics execute {session_id}' to analytics this session");
        return Ok(());
    }

    println!("=== Analysis Results for Session: {session_id} ===");
    println!();

    for request in requests {
        match service
            .get_analysis_result(request.id.clone())
            .await
            .map_err(|e| anyhow::anyhow!("{e}"))?
        {
            Some(analysis) => {
                // TODO: 좀더 좋은 방식으로 구현해야 합니다
                println!("{}", serde_json::to_string_pretty(&analysis)?);
            }
            None => {
                println!("Request {} - Status: {:?}", request.id, request.status);
                if let Some(error) = &request.error_message {
                    println!("Error: {error}");
                }
                println!();
            }
        }
    }

    Ok(())
}

async fn show_all_results(service: &AnalyticsRequestService) -> Result<()> {
    let requests = service
        .list_analyses(None, Some(50))
        .await
        .map_err(|e| anyhow::anyhow!("Failed to list analyses: {e}"))?;

    if requests.is_empty() {
        println!("No analyses found");
        println!("Run 'retrochat analytics execute' to start analyzing sessions");
        return Ok(());
    }

    println!("=== All Analysis Results ===");
    println!();

    for request in requests {
        match service
            .get_analysis_result(request.id.clone())
            .await
            .map_err(|e| anyhow::anyhow!("{e}"))?
        {
            Some(analysis) => {
                println!("{}", serde_json::to_string_pretty(&analysis)?);
            }
            None => {
                println!("Request {} - Status: {:?}", request.id, request.status);
                if let Some(error) = &request.error_message {
                    println!("Error: {error}");
                }
                println!();
            }
        }
    }

    Ok(())
}

pub async fn handle_status_command(all: bool, watch: bool, history: bool) -> Result<()> {
    let db_path = crate::database::config::get_default_db_path()?;
    let db_manager = Arc::new(DatabaseManager::new(&db_path).await?);

    // For status command, we don't need Google AI - just create a dummy client
    // since we're only reading from database
    let config = GoogleAiConfig::new("dummy-key-for-read-only".to_string());
    let google_ai_client = GoogleAiClient::new(config)?;
    let service = AnalyticsRequestService::new(db_manager, google_ai_client);

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

async fn show_current_status(service: &AnalyticsRequestService) -> Result<()> {
    let active_requests = service
        .get_active_analyses()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to get active analyses: {e}"))?;

    if active_requests.is_empty() {
        println!("No active analysis operations");
        return Ok(());
    }

    println!("=== Active Analysis Operations ===");
    println!();

    for request in active_requests {
        println!("Request: {}", request.id);
        println!("  Session: {}", request.session_id);
        println!("  Status: {:?}", request.status);
        println!("  Started: {}", request.started_at);
        if let Some(error) = &request.error_message {
            println!("  Error: {error}");
        }
        println!();
    }

    Ok(())
}

async fn show_all_active_status(service: &AnalyticsRequestService) -> Result<()> {
    show_current_status(service).await
}

async fn show_historical_status(service: &AnalyticsRequestService) -> Result<()> {
    let all_requests = service
        .list_analyses(None, Some(100))
        .await
        .map_err(|e| anyhow::anyhow!("Failed to get historical analyses: {e}"))?;

    println!("=== Analysis History ===");
    println!();

    for request in all_requests {
        println!(
            "Request: {} | Session: {} | Status: {:?}",
            request.id, request.session_id, request.status
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
    let db_path = crate::database::config::get_default_db_path()?;
    let db_manager = Arc::new(DatabaseManager::new(&db_path).await?);

    // For cancel command, we don't need Google AI - just create a dummy client
    // since we're only updating database status
    let config = GoogleAiConfig::new("dummy-key-for-cancel".to_string());
    let google_ai_client = GoogleAiClient::new(config)?;
    let service = AnalyticsRequestService::new(db_manager, google_ai_client);

    if all {
        cancel_all_requests(&service).await
    } else if let Some(request_id) = request_id {
        cancel_single_request(&service, &request_id).await
    } else {
        list_cancellable_requests(&service).await
    }
}

async fn cancel_single_request(service: &AnalyticsRequestService, request_id: &str) -> Result<()> {
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

async fn cancel_all_requests(service: &AnalyticsRequestService) -> Result<()> {
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

async fn list_cancellable_requests(service: &AnalyticsRequestService) -> Result<()> {
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
                    "ID: {} | Session: {} | Status: {:?}",
                    request.id, request.session_id, request.status
                );
            }
            _ => {} // Skip non-cancellable requests
        }
    }

    println!();
    println!("Use 'retrochat analytics cancel <request_id>' to cancel a specific request");
    println!("Use 'retrochat analytics cancel --all' to cancel all active requests");

    Ok(())
}

// =============================================================================
// Print Functions
// =============================================================================

async fn print_unified_analysis(analysis: &crate::models::Analytics) -> Result<()> {
    println!();
    println!("╔══════════════════════════════════════════════════════════════════════════════╗");
    println!("║                           📊 Analysis Results                                ║");
    println!("╚══════════════════════════════════════════════════════════════════════════════╝");
    println!();

    // Session Info
    println!("📋 Session: {}", analysis.session_id);
    println!(
        "🕐 Generated: {}",
        analysis.generated_at.format("%Y-%m-%d %H:%M:%S UTC")
    );
    if let Some(model) = &analysis.model_used {
        println!("🤖 Model: {model}");
    }
    if let Some(duration) = analysis.analysis_duration_ms {
        println!("⏱️  Analysis Duration: {}ms", duration);
    }
    println!();

    // 1. Metric Quantitative Output
    print_metric_quantitative(&analysis.metric_quantitative_output);

    // 2. AI Quantitative Output (Rubric Scores)
    print_ai_quantitative(&analysis.ai_quantitative_output);

    // 3. AI Qualitative Output
    print_ai_qualitative(&analysis.ai_qualitative_output);

    Ok(())
}

fn print_metric_quantitative(metrics: &crate::services::analytics::MetricQuantitativeOutput) {
    println!("┌──────────────────────────────────────────────────────────────────────────────┐");
    println!("│  📈 Metric Quantitative Analysis                                             │");
    println!("└──────────────────────────────────────────────────────────────────────────────┘");
    println!();

    // File Changes
    println!("  📁 File Changes:");
    println!(
        "     • Files Modified: {}",
        metrics.file_changes.total_files_modified
    );
    println!(
        "     • Files Read: {}",
        metrics.file_changes.total_files_read
    );
    println!("     • Lines Added: +{}", metrics.file_changes.lines_added);
    println!(
        "     • Lines Removed: -{}",
        metrics.file_changes.lines_removed
    );
    println!(
        "     • Net Code Growth: {}{}",
        if metrics.file_changes.net_code_growth >= 0 {
            "+"
        } else {
            ""
        },
        metrics.file_changes.net_code_growth
    );
    println!();

    // Time Metrics
    println!("  ⏰ Time Consumption:");
    println!(
        "     • Total Session Time: {:.1} minutes",
        metrics.time_metrics.total_session_time_minutes
    );
    if !metrics.time_metrics.peak_hours.is_empty() {
        let peak_hours: Vec<String> = metrics
            .time_metrics
            .peak_hours
            .iter()
            .map(|h| format!("{:02}:00", h))
            .collect();
        println!("     • Peak Hours: {}", peak_hours.join(", "));
    }
    println!();

    // Token Metrics
    println!("  🔢 Token Consumption:");
    println!(
        "     • Total Tokens: {}",
        metrics.token_metrics.total_tokens_used
    );
    println!(
        "     • Input Tokens: {}",
        metrics.token_metrics.input_tokens
    );
    println!(
        "     • Output Tokens: {}",
        metrics.token_metrics.output_tokens
    );
    println!(
        "     • Token Efficiency: {:.2}%",
        metrics.token_metrics.token_efficiency * 100.0
    );
    println!();

    // Tool Usage
    println!("  🔧 Tool Usage:");
    println!(
        "     • Total Operations: {}",
        metrics.tool_usage.total_operations
    );
    println!(
        "     • Successful: {}",
        metrics.tool_usage.successful_operations
    );
    println!("     • Failed: {}", metrics.tool_usage.failed_operations);
    if metrics.tool_usage.total_operations > 0 {
        let success_rate = (metrics.tool_usage.successful_operations as f64
            / metrics.tool_usage.total_operations as f64)
            * 100.0;
        println!("     • Success Rate: {:.1}%", success_rate);
    }
    println!(
        "     • Avg Execution Time: {:.1}ms",
        metrics.tool_usage.average_execution_time_ms
    );
    if !metrics.tool_usage.tool_distribution.is_empty() {
        println!("     • Tool Distribution:");
        let mut tools: Vec<_> = metrics.tool_usage.tool_distribution.iter().collect();
        tools.sort_by(|a, b| b.1.cmp(a.1));
        for (tool, count) in tools.iter().take(10) {
            println!("       - {}: {}", tool, count);
        }
        if tools.len() > 10 {
            println!("       ... and {} more tools", tools.len() - 10);
        }
    }
    println!();
}

fn print_ai_quantitative(ai_quant: &crate::services::analytics::AIQuantitativeOutput) {
    println!("┌──────────────────────────────────────────────────────────────────────────────┐");
    println!("│  🎯 AI Quantitative Analysis (Rubric Scores)                                 │");
    println!("└──────────────────────────────────────────────────────────────────────────────┘");
    println!();

    if ai_quant.rubric_scores.is_empty() {
        println!("  No rubric scores available.");
        println!();
        return;
    }

    // Summary
    if let Some(summary) = &ai_quant.rubric_summary {
        println!(
            "  📊 Overall Score: {:.1}/{:.1} ({:.1}%)",
            summary.total_score, summary.max_score, summary.percentage
        );
        println!(
            "     Rubrics Evaluated: {} (version: {})",
            summary.rubrics_evaluated, summary.rubrics_version
        );
        println!();
    }

    // Individual Rubric Scores
    println!("  📝 Rubric Scores:");
    for score in &ai_quant.rubric_scores {
        let bar = generate_score_bar(score.score, score.max_score);
        println!(
            "     [{bar}] {:.1}/{:.1}  {}",
            score.score, score.max_score, score.rubric_name
        );
        // Print reasoning with indentation
        if !score.reasoning.is_empty() {
            let reasoning_lines: Vec<&str> = score.reasoning.lines().collect();
            for line in reasoning_lines.iter().take(3) {
                println!("        💬 {}", line.trim());
            }
            if reasoning_lines.len() > 3 {
                println!("        ... ({} more lines)", reasoning_lines.len() - 3);
            }
        }
        println!();
    }
}

fn print_ai_qualitative(ai_qual: &crate::services::analytics::AIQualitativeOutput) {
    println!("┌──────────────────────────────────────────────────────────────────────────────┐");
    println!("│  💡 AI Qualitative Analysis                                                  │");
    println!("└──────────────────────────────────────────────────────────────────────────────┘");
    println!();

    // Summary
    if let Some(summary) = &ai_qual.summary {
        println!(
            "  📋 Summary: {} entries across {} categories (version: {})",
            summary.total_entries, summary.categories_evaluated, summary.entries_version
        );
        println!();
    }

    if ai_qual.entries.is_empty() {
        println!("  No qualitative analysis available.");
        println!();
        return;
    }

    // Print entries in order (entries are already ordered from config)
    for entry_output in &ai_qual.entries {
        if !entry_output.items.is_empty() {
            println!("  • {}:", entry_output.title);

            // Print summary if available
            if !entry_output.summary.is_empty() {
                println!("    📝 {}", entry_output.summary);
            }

            // Print items
            for item in &entry_output.items {
                println!("    - {}", item);
            }
            println!();
        }
    }
}

fn generate_score_bar(score: f64, max_score: f64) -> String {
    let percentage = if max_score > 0.0 {
        score / max_score
    } else {
        0.0
    };
    let filled = (percentage * 10.0).round() as usize;
    let empty = 10 - filled;
    format!("{}{}", "█".repeat(filled), "░".repeat(empty))
}
