use anyhow::Result;

use crate::database::connection::DatabaseManager;
use crate::services::analytics_service::AnalyticsService;

pub async fn handle_insights_command() -> Result<()> {
    let db_manager = DatabaseManager::new("retrochat.db")?;
    let analytics_service = AnalyticsService::new(db_manager);
    analytics_service.print_insights_summary().await
}

pub async fn handle_export_command(format: String, output_path: Option<String>) -> Result<()> {
    let db_manager = DatabaseManager::new("retrochat.db")?;
    let analytics_service = AnalyticsService::new(db_manager);
    let _response = analytics_service.export_data(&format, output_path).await?;
    Ok(())
}
