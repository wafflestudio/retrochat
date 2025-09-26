// Example showing how to use the new SQLx migration system
// This demonstrates the structured approach vs the old plain text approach

use anyhow::Result;
use chrono::Utc;
use retrochat::database::{DatabaseManager, MessageRepository, MigrationManager};
use retrochat::models::message::{Message, MessageRole};
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    // Create database manager with SQLx
    let db_manager = SqlxDatabaseManager::new("example.db").await?;

    // Get migration manager
    let migration_manager = SqlxMigrationManager::new(db_manager.pool().clone());

    // Check migration status
    let status = migration_manager.get_migration_status().await?;
    println!("Migration Status:");
    for migration in status {
        println!(
            "  Version {}: {} - Applied: {}",
            migration.version, migration.description, migration.applied
        );
    }

    // Validate database
    let is_valid = migration_manager.validate_database().await?;
    println!(
        "Database validation: {}",
        if is_valid { "PASSED" } else { "FAILED" }
    );

    // Create message repository
    let message_repo = MessageRepository::new(&db_manager);

    // Example: Create a message
    let message = Message {
        id: Uuid::new_v4(),
        session_id: Uuid::new_v4(),
        role: MessageRole::User,
        content: "Hello, this is a test message!".to_string(),
        timestamp: Utc::now(),
        token_count: Some(8),
        tool_calls: None,
        metadata: None,
        sequence_number: 1,
    };

    // Insert message using SQLx (type-safe query)
    message_repo.create(&message).await?;
    println!("Message created successfully");

    // Search for messages
    let search_results = message_repo.search_content("test", Some(10)).await?;
    println!("Found {} messages matching 'test'", search_results.len());

    // Get message by ID
    if let Some(retrieved_message) = message_repo.get_by_id(&message.id).await? {
        println!("Retrieved message: {}", retrieved_message.content);
    }

    // Health check
    db_manager.health_check().await?;
    println!("Database health check passed");

    // Close database
    db_manager.close().await?;

    Ok(())
}

// Example of how the old system worked vs new system:

/*
OLD SYSTEM (Plain Text SQL):
```rust
// In schema.rs - hardcoded SQL strings
conn.execute(
    "CREATE TABLE IF NOT EXISTS messages (
        id TEXT PRIMARY KEY,
        session_id TEXT NOT NULL,
        role TEXT NOT NULL CHECK (role IN ('User', 'Assistant', 'System')),
        content TEXT NOT NULL CHECK (length(content) > 0),
        timestamp TEXT NOT NULL,
        token_count INTEGER CHECK (token_count >= 0),
        tool_calls TEXT, -- JSON array
        metadata TEXT,   -- JSON object
        sequence_number INTEGER NOT NULL,
        FOREIGN KEY (session_id) REFERENCES chat_sessions(id) ON DELETE CASCADE,
        UNIQUE(session_id, sequence_number)
    )",
    [],
)?;
```

NEW SYSTEM (Structured SQLx):
```rust
// In migrations/001_initial_schema.sql - separate SQL file
CREATE TABLE IF NOT EXISTS messages (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    role TEXT NOT NULL CHECK (role IN ('User', 'Assistant', 'System')),
    content TEXT NOT NULL CHECK (length(content) > 0),
    timestamp TEXT NOT NULL,
    token_count INTEGER CHECK (token_count >= 0),
    tool_calls TEXT, -- JSON array
    metadata TEXT,   -- JSON object
    sequence_number INTEGER NOT NULL,
    FOREIGN KEY (session_id) REFERENCES chat_sessions(id) ON DELETE CASCADE,
    UNIQUE(session_id, sequence_number)
);

// In repository - type-safe queries
sqlx::query!(
    "INSERT INTO messages (id, session_id, role, content, timestamp, token_count, sequence_number) VALUES (?, ?, ?, ?, ?, ?, ?)",
    message.id.to_string(),
    message.session_id.to_string(),
    message.role.to_string(),
    message.content,
    message.timestamp.to_rfc3339(),
    message.token_count,
    message.sequence_number
)
.execute(&self.pool)
.await?;
```

BENEFITS:
1. ✅ Compile-time SQL validation
2. ✅ Type-safe queries with automatic parameter binding
3. ✅ Structured migration files instead of code closures
4. ✅ Better error handling and context
5. ✅ Automatic migration tracking
6. ✅ Easy rollback support
7. ✅ Better testing capabilities
8. ✅ Cleaner separation of concerns
*/
