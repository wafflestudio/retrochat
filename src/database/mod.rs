pub mod analysis_request_repo;
pub mod analytics_repo;
pub mod chat_session_repo;
pub mod connection;
pub mod message_repo;
pub mod migrations;
pub mod project_repo;
pub mod retrospection_repo;
pub mod schema;
pub mod seeds;

pub use analysis_request_repo::{AnalysisRequestRepository, QueueStatistics};
pub use analytics_repo::{
    AnalyticsRepository, DailyPoint, DailyUsageStats, HourlyActivity, ProviderTrend,
    SessionLengthDistribution,
};
pub use chat_session_repo::ChatSessionRepository;
pub use connection::{DatabaseManager, TableInfo};
pub use message_repo::MessageRepository;
pub use migrations::{Migration, MigrationManager, MigrationStatus};
pub use project_repo::ProjectRepository;
pub use retrospection_repo::{
    AnalysisStatistics, RetrospectionAnalysisRepository, StatusStatistics,
};
pub use schema::{create_schema, SCHEMA_VERSION};

// Main database structure for integration tests
pub struct Database {
    pub manager: DatabaseManager,
}

impl Database {
    pub fn new(db_path: &str) -> anyhow::Result<Self> {
        let manager = DatabaseManager::new(db_path)?;
        Ok(Self { manager })
    }

    pub fn new_in_memory() -> anyhow::Result<Self> {
        let manager = DatabaseManager::new(":memory:")?;
        Ok(Self { manager })
    }

    pub async fn setup(&self) -> anyhow::Result<()> {
        // Initialize schema and migrations
        self.manager.with_connection(create_schema)?;
        Ok(())
    }

    pub fn initialize(&self) -> anyhow::Result<()> {
        self.manager.with_connection(create_schema)?;
        Ok(())
    }
}
