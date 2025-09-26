# SQLx Migration Guide

This document outlines the migration from the current rusqlite-based system to SQLx for better structured migrations and type-safe queries.

## Overview

The current system uses:
- **rusqlite** with plain text SQL strings
- **Function-based migrations** with closures
- **Manual migration tracking**

The new SQLx system provides:
- **Compile-time query verification**
- **Type-safe queries** with automatic parameter binding
- **Structured migration files** (SQL files instead of code)
- **Better error handling** and context
- **Automatic migration tracking**

## Migration Structure

### 1. Migration Files

Migrations are now stored as SQL files in the `migrations/` directory:

```
migrations/
├── 001_initial_schema.sql
├── 002_add_message_tags.sql
└── 003_add_user_preferences.sql
```

Each migration file contains:
- **Up migration**: SQL to apply the changes
- **Down migration**: SQL to rollback (optional)
- **Version tracking**: Automatic via SQLx

### 2. Database Connection

**Old System:**
```rust
use rusqlite::{Connection, Result};

pub struct DatabaseManager {
    connection: Arc<Mutex<Connection>>,
}

impl DatabaseManager {
    pub fn new(db_path: impl AsRef<Path>) -> Result<Self> {
        let connection = Connection::open(&db_path)?;
        // Manual PRAGMA configuration
        connection.execute("PRAGMA foreign_keys = ON", [])?;
        // ... more PRAGMA statements
        Ok(Self { connection })
    }
}
```

**New System:**
```rust
use sqlx::{sqlite::SqlitePool, Pool, Sqlite};

pub struct SqlxDatabaseManager {
    pool: Pool<Sqlite>,
}

impl SqlxDatabaseManager {
    pub async fn new(db_path: impl AsRef<Path>) -> Result<Self> {
        let database_url = format!("sqlite://{}", db_path.display());
        let pool = SqlitePool::connect(&database_url).await?;
        
        // Automatic migrations
        sqlx::migrate!("./migrations").run(&pool).await?;
        
        Ok(Self { pool })
    }
}
```

### 3. Repository Layer

**Old System:**
```rust
pub fn create(&self, message: &Message) -> Result<()> {
    self.db.with_transaction(|conn| {
        conn.execute(
            "INSERT INTO messages (id, session_id, role, content, timestamp, token_count, sequence_number) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                message.id.to_string(),
                message.session_id.to_string(),
                message.role.to_string(),
                message.content,
                message.timestamp.to_rfc3339(),
                message.token_count,
                message.sequence_number
            ],
        )?;
        Ok(())
    })
}
```

**New System:**
```rust
pub async fn create(&self, message: &Message) -> Result<()> {
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
    
    Ok(())
}
```

## Benefits

### 1. Compile-time Safety
- SQL queries are validated at compile time
- Parameter types are checked automatically
- No more runtime SQL syntax errors

### 2. Type Safety
- Automatic parameter binding
- Type inference for query results
- Better IDE support with autocomplete

### 3. Structured Migrations
- SQL files instead of code closures
- Version control friendly
- Easy to review and understand
- Automatic up/down migration support

### 4. Better Error Handling
- More descriptive error messages
- Better context for debugging
- Async/await support

### 5. Testing
- Easier to mock and test
- Better integration test support
- In-memory database support

## Migration Steps

### Phase 1: Setup (✅ Completed)
- [x] Add SQLx dependencies
- [x] Create migration directory structure
- [x] Convert initial schema to SQL migration file
- [x] Create SQLx database manager
- [x] Create SQLx migration manager

### Phase 2: Repository Migration (🔄 In Progress)
- [ ] Convert MessageRepository to SQLx
- [ ] Convert ChatSessionRepository to SQLx
- [ ] Convert ProjectRepository to SQLx
- [ ] Convert AnalyticsRepository to SQLx

### Phase 3: Integration (⏳ Pending)
- [ ] Update service layer to use async repositories
- [ ] Update CLI commands to use async database operations
- [ ] Update tests to work with SQLx
- [ ] Add migration rollback support

### Phase 4: Cleanup (⏳ Pending)
- [ ] Remove old rusqlite code
- [ ] Update documentation
- [ ] Performance testing and optimization

## Usage Examples

### Creating a New Migration

1. Create SQL file: `migrations/003_add_user_preferences.sql`
2. Add up migration SQL
3. Add down migration SQL (optional)
4. Run migrations: `sqlx migrate run`

### Using Type-Safe Queries

```rust
// Simple query
let user = sqlx::query!(
    "SELECT * FROM users WHERE email = ?",
    email
)
.fetch_optional(&pool)
.await?;

// Complex query with joins
let sessions = sqlx::query!(
    r#"
    SELECT cs.*, p.name as project_name
    FROM chat_sessions cs
    LEFT JOIN projects p ON cs.project_name = p.name
    WHERE cs.provider = ? AND cs.start_time >= ?
    ORDER BY cs.start_time DESC
    LIMIT ?
    "#,
    provider,
    start_date,
    limit
)
.fetch_all(&pool)
.await?;
```

### Migration Management

```rust
let migration_manager = SqlxMigrationManager::new(pool.clone());

// Check status
let status = migration_manager.get_migration_status().await?;

// Migrate to latest
migration_manager.migrate_to_latest().await?;

// Rollback to specific version
migration_manager.migrate_to_version(1, 3).await?;

// Reset database
migration_manager.reset_database().await?;
```

## Local Executable Considerations

Since this is a local executable (not a server), SQLx provides several advantages:

1. **No external dependencies**: SQLx works with embedded SQLite
2. **Better performance**: Connection pooling and async operations
3. **Easier deployment**: Single binary with embedded database
4. **Better development experience**: Compile-time validation
5. **Easier testing**: In-memory database support

## Rollback Strategy

If issues arise during migration:

1. **Keep old system**: Both systems can coexist during transition
2. **Feature flags**: Use feature flags to switch between systems
3. **Gradual migration**: Migrate one repository at a time
4. **Testing**: Comprehensive testing before full migration

## Performance Considerations

- **Connection pooling**: SQLx provides better connection management
- **Async operations**: Better resource utilization
- **Compile-time optimization**: Queries are optimized at compile time
- **Memory usage**: More efficient memory usage with async/await

## Conclusion

The migration to SQLx provides significant improvements in:
- **Developer experience**: Better tooling and error messages
- **Code quality**: Type safety and compile-time validation
- **Maintainability**: Structured migrations and cleaner code
- **Performance**: Better async support and connection pooling
- **Testing**: Easier mocking and integration testing

The structured approach with SQL migration files is much more maintainable than the current function-based approach, especially for a local executable where we want to keep things simple and reliable.