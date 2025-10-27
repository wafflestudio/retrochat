use anyhow::Result;
use chrono::Utc;
use std::path::PathBuf;

use crate::database::DatabaseManager;
use crate::services::analytics_service::AnalyticsService;
use crate::services::google_ai::GoogleAiClient;

pub async fn handle_insights_command() -> Result<()> {
    let db_path = crate::database::config::get_default_db_path()?;
    let db_manager = DatabaseManager::new(&db_path).await?;
    let analytics_service = AnalyticsService::new(db_manager);
    print_insights_summary(&analytics_service).await
}

async fn print_insights_summary(analytics_service: &AnalyticsService) -> Result<()> {
    let insights = analytics_service.generate_usage_insights().await?;

    println!("\nUsage Insights Summary");
    println!("======================");
    println!("Total Sessions: {}", insights.total_sessions);
    println!("Total Messages: {}", insights.total_messages);
    println!("Total Tokens: {}", insights.total_tokens);

    if !insights.date_range.start_date.is_empty() && !insights.date_range.end_date.is_empty() {
        println!(
            "Date Range: {} to {} ({} days)",
            insights.date_range.start_date, insights.date_range.end_date, insights.span_days
        );
    }

    println!("\nProvider Breakdown:");
    for (provider, stats) in &insights.provider_breakdown {
        println!(
            "  {}: {} sessions ({:.1}%)",
            provider, stats.sessions, stats.percentage_of_total
        );
    }

    println!("\nFor detailed analysis, use: retrochat analyze export json");

    Ok(())
}

pub async fn handle_export_command(format: String, output_path: Option<String>) -> Result<()> {
    let db_path = crate::database::config::get_default_db_path()?;
    let db_manager = DatabaseManager::new(&db_path).await?;
    let analytics_service = AnalyticsService::new(db_manager);

    let path = match output_path {
        Some(p) => PathBuf::from(p),
        None => {
            let filename = format!(
                "retrochat_export_{}.{}",
                Utc::now().format("%Y%m%d_%H%M%S"),
                format
            );
            PathBuf::from(filename)
        }
    };

    let _response = analytics_service
        .export_data(&format, &path.to_string_lossy())
        .await?;
    Ok(())
}

// =============================================================================
// Unified Analysis Command
// =============================================================================

/// Handle unified analysis command - combines all analysis types
pub async fn handle_analyze_command(session_id: Option<String>) -> Result<()> {
    let db_path = crate::database::config::get_default_db_path()?;
    let db_manager = DatabaseManager::new(&db_path).await?;

    // Try to initialize Google AI client if API key is available
    let analytics_service = if let Ok(api_key) = std::env::var("GOOGLE_AI_API_KEY") {
        let config = crate::services::google_ai::GoogleAiConfig {
            api_key,
            ..Default::default()
        };
        let google_ai_client = GoogleAiClient::new(config)?;
        AnalyticsService::new(db_manager).with_google_ai(google_ai_client)
    } else {
        AnalyticsService::new(db_manager)
    };

    if let Some(session_id) = session_id {
        // Analyze specific session with all analysis types
        let analysis = analytics_service
            .analyze_session_comprehensive(&session_id)
            .await?;
        print_unified_analysis(&analysis).await?;
    } else {
        // Show usage insights if no session ID provided
        print_insights_summary(&analytics_service).await?;
    }

    Ok(())
}

// =============================================================================
// Print Functions
// =============================================================================

async fn print_unified_analysis(analysis: &crate::services::ComprehensiveAnalysis) -> Result<()> {
    println!("\n🔍 Session Analysis Report");
    println!("==========================");
    println!("Session ID: {}", analysis.session_id);
    println!(
        "Generated: {}",
        analysis.generated_at.format("%Y-%m-%d %H:%M:%S UTC")
    );

    // =============================================================================
    // 1. QUANTITATIVE SCORES
    // =============================================================================
    println!("\n📊 QUANTITATIVE SCORES");
    println!("─────────────────────");
    println!(
        "  Overall Score: {:.1}/100",
        analysis.quantitative_output.overall_score
    );
    println!(
        "  Code Quality: {:.1}/100",
        analysis.quantitative_output.code_quality_score
    );
    println!(
        "  Productivity: {:.1}/100",
        analysis.quantitative_output.productivity_score
    );
    println!(
        "  Efficiency: {:.1}/100",
        analysis.quantitative_output.efficiency_score
    );
    println!(
        "  Collaboration: {:.1}/100",
        analysis.quantitative_output.collaboration_score
    );
    println!(
        "  Learning: {:.1}/100",
        analysis.quantitative_output.learning_score
    );

    // =============================================================================
    // 2. QUANTITATIVE METRICS
    // =============================================================================
    println!("\n📈 QUANTITATIVE METRICS");
    println!("─────────────────────");

    // File changes
    println!("\n📁 File Changes:");
    println!(
        "  Files Modified: {}",
        analysis
            .quantitative_input
            .file_changes
            .total_files_modified
    );
    println!(
        "  Files Read: {}",
        analysis.quantitative_input.file_changes.total_files_read
    );
    println!(
        "  Lines Added: {}",
        analysis.quantitative_input.file_changes.lines_added
    );
    println!(
        "  Lines Removed: {}",
        analysis.quantitative_input.file_changes.lines_removed
    );
    println!(
        "  Net Code Growth: {}",
        analysis.quantitative_input.file_changes.net_code_growth
    );
    println!(
        "  Refactoring Operations: {}",
        analysis
            .quantitative_input
            .file_changes
            .refactoring_operations
    );

    // Time metrics
    println!("\n⏱️ Time Metrics:");
    println!(
        "  Session Duration: {:.1} minutes",
        analysis
            .quantitative_input
            .time_metrics
            .total_session_time_minutes
    );
    println!(
        "  Peak Hours: {:?}",
        analysis.quantitative_input.time_metrics.peak_hours
    );

    // Token metrics
    println!("\n🔤 Token Metrics:");
    println!(
        "  Total Tokens: {}",
        analysis.quantitative_input.token_metrics.total_tokens_used
    );
    println!(
        "  Input Tokens: {}",
        analysis.quantitative_input.token_metrics.input_tokens
    );
    println!(
        "  Output Tokens: {}",
        analysis.quantitative_input.token_metrics.output_tokens
    );
    println!(
        "  Token Efficiency: {:.2}",
        analysis.quantitative_input.token_metrics.token_efficiency
    );

    // Tool usage
    println!("\n🛠️ Tool Usage:");
    println!(
        "  Total Operations: {}",
        analysis.quantitative_input.tool_usage.total_operations
    );
    println!(
        "  Successful: {}",
        analysis.quantitative_input.tool_usage.successful_operations
    );
    println!(
        "  Failed: {}",
        analysis.quantitative_input.tool_usage.failed_operations
    );

    if !analysis
        .quantitative_input
        .tool_usage
        .tool_distribution
        .is_empty()
    {
        println!("  Tool Distribution:");
        for (tool, count) in &analysis.quantitative_input.tool_usage.tool_distribution {
            println!("    {tool}: {count}");
        }
    }

    // =============================================================================
    // 3. PROCESSED STATISTICS
    // =============================================================================
    println!("\n⚙️ PROCESSED STATISTICS");
    println!("─────────────────────");

    // Session metrics
    println!("\n📊 Session Metrics:");
    println!(
        "  Total Sessions: {}",
        analysis.processed_output.session_metrics.total_sessions
    );
    println!(
        "  Avg Duration: {:.1} min",
        analysis
            .processed_output
            .session_metrics
            .average_session_duration_minutes
    );
    println!(
        "  Consistency Score: {:.1}%",
        analysis
            .processed_output
            .session_metrics
            .session_consistency_score
            * 100.0
    );

    // Token metrics
    println!("\n🔤 Token Statistics:");
    println!(
        "  Total Tokens: {}",
        analysis.processed_output.token_metrics.total_tokens
    );
    println!(
        "  Tokens/Hour: {:.1}",
        analysis.processed_output.token_metrics.tokens_per_hour
    );
    println!(
        "  I/O Ratio: {:.2}",
        analysis.processed_output.token_metrics.input_output_ratio
    );
    println!(
        "  Efficiency Score: {:.1}%",
        analysis
            .processed_output
            .token_metrics
            .token_efficiency_score
            * 100.0
    );
    println!(
        "  Cost Estimate: ${:.4}",
        analysis.processed_output.token_metrics.cost_estimate
    );

    // Code metrics
    println!("\n💻 Code Statistics:");
    println!(
        "  Net Lines Changed: {}",
        analysis
            .processed_output
            .code_change_metrics
            .net_lines_changed
    );
    println!(
        "  Files/Session: {:.1}",
        analysis
            .processed_output
            .code_change_metrics
            .files_per_session
    );
    println!(
        "  Lines/Hour: {:.1}",
        analysis.processed_output.code_change_metrics.lines_per_hour
    );
    println!(
        "  Refactoring Ratio: {:.1}%",
        analysis
            .processed_output
            .code_change_metrics
            .refactoring_ratio
            * 100.0
    );
    println!(
        "  Code Velocity: {:.1}",
        analysis.processed_output.code_change_metrics.code_velocity
    );

    // Time efficiency
    println!("\n⏱️ Time Efficiency:");
    println!(
        "  Productivity Score: {:.1}%",
        analysis
            .processed_output
            .time_efficiency_metrics
            .productivity_score
            * 100.0
    );
    println!(
        "  Context Switching Cost: {:.1}%",
        analysis
            .processed_output
            .time_efficiency_metrics
            .context_switching_cost
            * 100.0
    );
    println!(
        "  Deep Work Ratio: {:.1}%",
        analysis
            .processed_output
            .time_efficiency_metrics
            .deep_work_ratio
            * 100.0
    );
    println!(
        "  Time Utilization: {:.1}%",
        analysis
            .processed_output
            .time_efficiency_metrics
            .time_utilization
            * 100.0
    );

    // =============================================================================
    // 4. QUALITATIVE INSIGHTS
    // =============================================================================
    println!("\n💭 QUALITATIVE INSIGHTS");
    println!("─────────────────────");

    // Insights
    println!("\n💡 Key Insights:");
    for insight in &analysis.qualitative_output.insights {
        println!(
            "  [{}] {}: {}",
            insight.category, insight.title, insight.description
        );
        println!("    Confidence: {:.1}%", insight.confidence * 100.0);
    }

    // Good patterns
    if !analysis.qualitative_output.good_patterns.is_empty() {
        println!("\n✅ Good Patterns:");
        for pattern in &analysis.qualitative_output.good_patterns {
            println!(
                "  • {} ({} times): {}",
                pattern.pattern_name, pattern.frequency, pattern.description
            );
            println!("    Impact: {}", pattern.impact);
        }
    }

    // Improvement areas
    if !analysis.qualitative_output.improvement_areas.is_empty() {
        println!("\n🔧 Improvement Areas:");
        for area in &analysis.qualitative_output.improvement_areas {
            println!("  • {} [{}]", area.area_name, area.priority);
            println!("    Current: {}", area.current_state);
            println!("    Suggestion: {}", area.suggested_improvement);
            println!("    Expected Impact: {}", area.expected_impact);
        }
    }

    // Recommendations
    if !analysis.qualitative_output.recommendations.is_empty() {
        println!("\n🎯 Recommendations:");
        for rec in &analysis.qualitative_output.recommendations {
            println!("  • {}: {}", rec.title, rec.description);
            println!("    Impact Score: {:.1}/10", rec.impact_score * 10.0);
            println!("    Difficulty: {}", rec.implementation_difficulty);
        }
    }

    // Learning observations
    if !analysis.qualitative_output.learning_observations.is_empty() {
        println!("\n📚 Learning Observations:");
        for obs in &analysis.qualitative_output.learning_observations {
            println!("  • {} ({})", obs.observation, obs.skill_area);
            println!("    Progress: {}", obs.progress_indicator);
            if !obs.next_steps.is_empty() {
                println!("    Next Steps: {}", obs.next_steps.join(", "));
            }
        }
    }

    println!("\n{}", "=".repeat(50));
    println!("Analysis complete! Use 'retrochat query sessions' to see other sessions.");
    println!("{}", "=".repeat(50));

    Ok(())
}
