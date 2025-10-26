pub mod analytics_repo;
pub mod chat_session_repo;
pub mod config;
pub mod connection;
pub mod flowchart_repo;
pub mod message_repo;
pub mod migrations;
pub mod project_repo;
pub mod retrospect_request_repo;
pub mod retrospection_repo;
pub mod schema;

// Main repositories (now using SQLx)
pub use analytics_repo::{
    AnalyticsRepository, DailyPoint, DailyUsageStats, HourlyActivity, ProviderTrend,
    SessionLengthDistribution,
};
pub use chat_session_repo::ChatSessionRepository;
pub use connection::DatabaseManager;
pub use flowchart_repo::FlowchartRepository;
pub use message_repo::MessageRepository;
pub use migrations::{MigrationManager, MigrationStatus};
pub use project_repo::ProjectRepository;
pub use retrospect_request_repo::RetrospectRequestRepository;
pub use retrospection_repo::RetrospectionRepository;
pub use schema::{create_schema, SCHEMA_VERSION};

// Main database structure (now using SQLx by default)
pub struct Database {
    pub manager: DatabaseManager,
}

impl Database {
    pub async fn new(db_path: &str) -> anyhow::Result<Self> {
        let manager = DatabaseManager::new(db_path).await?;
        Ok(Self { manager })
    }

    pub async fn new_in_memory() -> anyhow::Result<Self> {
        let manager = DatabaseManager::open_in_memory().await?;
        Ok(Self { manager })
    }

    pub async fn setup(&self) -> anyhow::Result<()> {
        // Migrations are automatically run during initialization
        Ok(())
    }

    pub async fn initialize(&self) -> anyhow::Result<()> {
        // Migrations are automatically run during initialization
        Ok(())
    }

    // Repository getters
    pub fn message_repo(&self) -> MessageRepository {
        MessageRepository::new(&self.manager)
    }

    pub fn project_repo(&self) -> ProjectRepository {
        ProjectRepository::new(&self.manager)
    }

    pub fn chat_session_repo(&self) -> ChatSessionRepository {
        ChatSessionRepository::new(&self.manager)
    }

    pub fn analytics_repo(&self) -> AnalyticsRepository {
        AnalyticsRepository::new(&self.manager)
    }

    pub fn retrospect_request_repo(&self) -> RetrospectRequestRepository {
        RetrospectRequestRepository::new(std::sync::Arc::new(self.manager.clone()))
    }

    pub fn retrospection_repo(&self) -> RetrospectionRepository {
        RetrospectionRepository::new(std::sync::Arc::new(self.manager.clone()))
    }

    pub fn flowchart_repo(&self) -> FlowchartRepository {
        FlowchartRepository::new(std::sync::Arc::new(self.manager.clone()))
    }

    pub fn migration_manager(&self) -> MigrationManager {
        MigrationManager::new(self.manager.pool().clone())
    }
}
