pub mod analytics_repo;
pub mod analytics_request_repo;
pub mod chat_session_repo;
pub mod config;
pub mod connection;
pub mod message_repo;
pub mod migrations;
pub mod project_repo;
pub mod schema;
pub mod session_summary_repo;
pub mod tool_operation_repo;
pub mod turn_summary_repo;

// Main repositories (now using SQLx)
pub use analytics_repo::AnalyticsRepository;
pub use analytics_request_repo::AnalyticsRequestRepository;
pub use chat_session_repo::ChatSessionRepository;
pub use connection::DatabaseManager;
pub use message_repo::MessageRepository;
pub use migrations::{MigrationManager, MigrationStatus};
pub use project_repo::ProjectRepository;
pub use schema::{create_schema, SCHEMA_VERSION};
pub use session_summary_repo::SessionSummaryRepository;
pub use tool_operation_repo::ToolOperationRepository;
pub use turn_summary_repo::TurnSummaryRepository;

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

    pub fn tool_operation_repo(&self) -> ToolOperationRepository {
        ToolOperationRepository::new(&self.manager)
    }

    pub fn turn_summary_repo(&self) -> TurnSummaryRepository {
        TurnSummaryRepository::new(&self.manager)
    }

    pub fn session_summary_repo(&self) -> SessionSummaryRepository {
        SessionSummaryRepository::new(&self.manager)
    }

    pub fn migration_manager(&self) -> MigrationManager {
        MigrationManager::new(self.manager.pool().clone())
    }
}
