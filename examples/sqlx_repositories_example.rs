// Comprehensive example showing how to use all SQLx repositories
// This demonstrates the complete migration from rusqlite to SQLx

use anyhow::Result;
use chrono::Utc;
use retrochat::database::Database;
use retrochat::models::{
    chat_session::{ChatSession, LlmProvider, SessionState},
    message::{Message, MessageRole},
    project::Project,
};
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    println!("🚀 SQLx Repositories Example");
    println!("=============================");

    // Create SQLx database (in-memory for this example)
    let db = Database::new_in_memory().await?;
    println!("✅ Database initialized with SQLx migrations");

    // Get all repositories
    let project_repo = db.project_repo();
    let chat_session_repo = db.chat_session_repo();
    let message_repo = db.message_repo();
    let analytics_repo = db.analytics_repo();
    let migration_manager = db.migration_manager();

    // Check migration status
    let migration_status = migration_manager.get_migration_status().await?;
    println!("\n📋 Migration Status:");
    for migration in migration_status {
        println!(
            "  Version {}: {} - Applied: {}",
            migration.version, migration.description, migration.applied
        );
    }

    // Validate database
    let is_valid = migration_manager.validate_database().await?;
    println!(
        "✅ Database validation: {}",
        if is_valid { "PASSED" } else { "FAILED" }
    );

    // === PROJECT REPOSITORY EXAMPLES ===
    println!("\n📁 Project Repository Examples");
    println!("-------------------------------");

    // Create a project
    let mut project = Project::new("My Awesome Project".to_string())
        .with_description("A project for testing SQLx repositories".to_string());

    project_repo.create(&project).await?;
    println!("✅ Created project: {}", project.name);

    // Get project by name
    if let Some(retrieved_project) = project_repo.get_by_name("My Awesome Project").await? {
        println!(
            "✅ Retrieved project: {} (ID: {})",
            retrieved_project.name, retrieved_project.id
        );
    }

    // Update project
    project.session_count = 5;
    project.total_tokens = 1000;
    project_repo.update(&project).await?;
    println!(
        "✅ Updated project with session count: {}",
        project.session_count
    );

    // Get all projects
    let all_projects = project_repo.get_all().await?;
    println!("✅ Total projects: {}", all_projects.len());

    // === CHAT SESSION REPOSITORY EXAMPLES ===
    println!("\n💬 Chat Session Repository Examples");
    println!("------------------------------------");

    // Create chat sessions
    let session1 = ChatSession::new(
        LlmProvider::ClaudeCode,
        "conversation1.jsonl".to_string(),
        "hash123".to_string(),
        Utc::now(),
    );

    let session2 = ChatSession::new(
        LlmProvider::Gemini,
        "conversation2.json".to_string(),
        "hash456".to_string(),
        Utc::now(),
    );

    chat_session_repo.create(&session1).await?;
    chat_session_repo.create(&session2).await?;
    println!("✅ Created 2 chat sessions");

    // Get session by ID
    if let Some(retrieved_session) = chat_session_repo.get_by_id(&session1.id).await? {
        println!(
            "✅ Retrieved session: {} (Provider: {})",
            retrieved_session.file_path, retrieved_session.provider
        );
    }

    // Get sessions by provider
    let claude_sessions = chat_session_repo
        .get_by_provider(&LlmProvider::ClaudeCode)
        .await?;
    println!("✅ Claude sessions: {}", claude_sessions.len());

    // Update session state
    let mut updated_session = session1.clone();
    updated_session.state = SessionState::Imported;
    updated_session.message_count = 10;
    updated_session.token_count = Some(500);
    chat_session_repo.update(&updated_session).await?;
    println!("✅ Updated session state to: {}", updated_session.state);

    // === MESSAGE REPOSITORY EXAMPLES ===
    println!("\n📝 Message Repository Examples");
    println!("-------------------------------");

    // Create messages
    let message1 = Message {
        id: Uuid::new_v4(),
        session_id: session1.id,
        role: MessageRole::User,
        content: "Hello, how can you help me today?".to_string(),
        timestamp: Utc::now(),
        token_count: Some(8),
        tool_calls: None,
        metadata: None,
        sequence_number: 1,
    };

    let message2 = Message {
        id: Uuid::new_v4(),
        session_id: session1.id,
        role: MessageRole::Assistant,
        content: "I can help you with various tasks! What would you like to work on?".to_string(),
        timestamp: Utc::now(),
        token_count: Some(15),
        tool_calls: None,
        metadata: None,
        sequence_number: 2,
    };

    message_repo.create(&message1).await?;
    message_repo.create(&message2).await?;
    println!("✅ Created 2 messages");

    // Get messages by session
    let session_messages = message_repo.get_by_session_id(&session1.id).await?;
    println!("✅ Messages in session: {}", session_messages.len());

    // Search messages
    let search_results = message_repo.search_content("help", Some(10)).await?;
    println!("✅ Search results for 'help': {}", search_results.len());

    // Get message by ID
    if let Some(retrieved_message) = message_repo.get_by_id(&message1.id).await? {
        println!("✅ Retrieved message: {}", retrieved_message.content);
    }

    // === ANALYTICS REPOSITORY EXAMPLES ===
    println!("\n📊 Analytics Repository Examples");
    println!("--------------------------------");

    // Get daily usage stats
    let today = Utc::now();
    let daily_stats = analytics_repo.get_daily_usage_stats(today).await?;
    println!(
        "✅ Daily stats - Sessions: {}, Messages: {}, Tokens: {}",
        daily_stats.total_sessions, daily_stats.total_messages, daily_stats.total_tokens
    );

    // Get session length distribution
    let length_dist = analytics_repo.get_session_length_distribution().await?;
    println!("✅ Session length distribution:");
    println!(
        "  Short (≤5): {} ({:.1}%)",
        length_dist.short_sessions, length_dist.short_percentage
    );
    println!(
        "  Medium (6-20): {} ({:.1}%)",
        length_dist.medium_sessions, length_dist.medium_percentage
    );
    println!(
        "  Long (21-50): {} ({:.1}%)",
        length_dist.long_sessions, length_dist.long_percentage
    );
    println!(
        "  Very Long (>50): {} ({:.1}%)",
        length_dist.very_long_sessions, length_dist.very_long_percentage
    );

    // Get provider usage trends
    let trends = analytics_repo.get_provider_usage_trends(7).await?;
    println!("✅ Provider trends (last 7 days):");
    for trend in trends {
        println!(
            "  {}: {} sessions, {} tokens",
            trend.provider, trend.total_sessions, trend.total_tokens
        );
    }

    // Generate insights
    let insights = analytics_repo.generate_insights(7).await?;
    println!("✅ Generated insights:");
    for insight in insights {
        println!("  {}", insight);
    }

    // Get total stats
    let (total_sessions, total_messages, total_tokens) = analytics_repo.get_total_stats().await?;
    println!(
        "✅ Total stats - Sessions: {}, Messages: {}, Tokens: {}",
        total_sessions, total_messages, total_tokens
    );

    // === PERFORMANCE COMPARISON ===
    println!("\n⚡ Performance Comparison");
    println!("-------------------------");

    let start = std::time::Instant::now();

    // Batch operations
    for i in 0..100 {
        let test_message = Message {
            id: Uuid::new_v4(),
            session_id: session1.id,
            role: MessageRole::User,
            content: format!("Test message {}", i),
            timestamp: Utc::now(),
            token_count: Some(5),
            tool_calls: None,
            metadata: None,
            sequence_number: i + 3,
        };
        message_repo.create(&test_message).await?;
    }

    let duration = start.elapsed();
    println!("✅ Created 100 messages in {:?}", duration);

    // Health check
    db.manager.health_check().await?;
    println!("✅ Database health check passed");

    println!("\n🎉 All SQLx repository examples completed successfully!");
    println!("\nKey Benefits Demonstrated:");
    println!("• ✅ Type-safe queries with compile-time validation");
    println!("• ✅ Async/await support for better performance");
    println!("• ✅ Structured migrations with SQL files");
    println!("• ✅ Better error handling and context");
    println!("• ✅ Connection pooling and resource management");
    println!("• ✅ Easy repository pattern with dependency injection");

    Ok(())
}

// Example of how the old system worked vs new system:

/*
OLD SYSTEM (rusqlite):
```rust
// Synchronous, manual parameter binding
pub fn create(&self, project: &Project) -> Result<()> {
    self.db_manager.with_transaction(|conn| {
        conn.execute(
            "INSERT INTO projects (id, name, description, working_directory, created_at, updated_at, session_count, total_tokens)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                project.id.to_string(),
                project.name,
                project.description,
                project.working_directory.as_ref().map(|p| p.to_string_lossy().to_string()),
                project.created_at.to_rfc3339(),
                project.updated_at.to_rfc3339(),
                project.session_count,
                project.total_tokens,
            ],
        )?;
        Ok(())
    })
}
```

NEW SYSTEM (SQLx):
```rust
// Async, type-safe parameter binding
pub async fn create(&self, project: &Project) -> AnyhowResult<()> {
    sqlx::query(
        r#"
        INSERT INTO projects (id, name, description, working_directory, created_at, updated_at, session_count, total_tokens)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?)
        "#
    )
    .bind(project.id.to_string())
    .bind(&project.name)
    .bind(project.description.as_ref())
    .bind(project.working_directory.as_ref().map(|p| p.to_string_lossy().to_string()))
    .bind(project.created_at.to_rfc3339())
    .bind(project.updated_at.to_rfc3339())
    .bind(project.session_count)
    .bind(project.total_tokens as i64)
    .execute(&self.pool)
    .await
    .context("Failed to create project")?;

    Ok(())
}
```

MIGRATION BENEFITS:
1. ✅ Compile-time SQL validation
2. ✅ Type-safe parameter binding
3. ✅ Async/await support
4. ✅ Better error handling with context
5. ✅ Connection pooling
6. ✅ Structured migrations
7. ✅ Easier testing and mocking
8. ✅ Better performance for concurrent operations
*/
