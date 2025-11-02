use anyhow::Result;
use std::sync::Arc;

use crate::database::{DatabaseManager, RetrospectRequestRepository, RetrospectionRepository};
use crate::models::{RetrospectRequest, Retrospection};
use crate::services::analytics::formatters::{AnalyticsFormatter, OutputFormat};
use crate::services::analytics_service::AnalyticsService;
use crate::services::google_ai::GoogleAiClient;

pub async fn handle_insights_command() -> Result<()> {
    let db_path = crate::database::config::get_default_db_path()?;
    let db_manager = Arc::new(DatabaseManager::new(&db_path).await?);
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

    Ok(())
}

// =============================================================================
// Unified Analysis Command
// =============================================================================

/// Handle unified analysis command - combines all analysis types
pub async fn handle_analyze_command(
    session_id: Option<String>,
    format: String,
    plain: bool,
) -> Result<()> {
    let db_path = crate::database::config::get_default_db_path()?;
    let db_manager = Arc::new(DatabaseManager::new(&db_path).await?);

    // Determine output format
    let output_format = if plain {
        OutputFormat::Plain
    } else {
        OutputFormat::parse(&format)
    };

    // Try to initialize Google AI client if API key is available
    let analytics_service = if let Ok(api_key) = std::env::var("GOOGLE_AI_API_KEY") {
        let config = crate::services::google_ai::GoogleAiConfig {
            api_key,
            ..Default::default()
        };
        let google_ai_client = GoogleAiClient::new(config)?;
        AnalyticsService::new(db_manager.clone()).with_google_ai(google_ai_client)
    } else {
        AnalyticsService::new(db_manager.clone())
    };

    if let Some(session_id) = session_id {
        // Create a retrospect request first
        let request_repo = RetrospectRequestRepository::new(db_manager.clone());
        let request =
            RetrospectRequest::new(session_id.clone(), Some("analytics-cli".to_string()), None);
        request_repo
            .create(&request)
            .await
            .map_err(|e| anyhow::anyhow!("{e}"))?;

        // Analyze specific session with all analysis types
        let start_time = std::time::Instant::now();
        let analysis = analytics_service
            .analyze_session_comprehensive(&session_id)
            .await?;
        let analysis_duration_ms = start_time.elapsed().as_millis() as i64;

        // Save analysis to database
        let retrospection = Retrospection::from_comprehensive_analysis(
            request.id.clone(),
            analysis.clone(),
            Some("gemini-pro".to_string()),
            Some(analysis_duration_ms),
        );

        let retrospection_repo = RetrospectionRepository::new(db_manager.clone());
        retrospection_repo
            .create(&retrospection)
            .await
            .map_err(|e| anyhow::anyhow!("{e}"))?;

        // Mark request as completed
        let mut completed_request = request;
        completed_request.mark_completed();
        request_repo
            .update(&completed_request)
            .await
            .map_err(|e| anyhow::anyhow!("{e}"))?;

        println!("Analysis saved to database (ID: {})\n", retrospection.id);

        // Print analysis
        print_unified_analysis(&analysis, output_format).await?;
    } else {
        // Show usage insights if no session ID provided
        print_insights_summary(&analytics_service).await?;
    }

    Ok(())
}

// =============================================================================
// Print Functions
// =============================================================================

async fn print_unified_analysis(
    analysis: &crate::services::ComprehensiveAnalysis,
    output_format: OutputFormat,
) -> Result<()> {
    let formatter = AnalyticsFormatter::new(output_format);
    formatter.print_analysis(analysis)
}
