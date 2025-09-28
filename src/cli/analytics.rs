use anyhow::Result;

use crate::database::DatabaseManager;
use crate::services::analytics_service::AnalyticsService;

pub async fn handle_insights_command() -> Result<()> {
    let db_manager = DatabaseManager::new("retrochat.db").await?;
    let analytics_service = AnalyticsService::new(db_manager);
    print_insights_summary(&analytics_service).await
}

async fn print_insights_summary(analytics_service: &AnalyticsService) -> Result<()> {
    let insights = analytics_service.generate_insights().await?;

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
    let db_manager = DatabaseManager::new("retrochat.db").await?;
    let analytics_service = AnalyticsService::new(db_manager);
    let _response = analytics_service.export_data(&format, output_path).await?;
    Ok(())
}
