use rusqlite::{Connection, Result};
use std::collections::HashMap;
use tracing::{error, info, warn};

use super::schema::{create_schema, SCHEMA_VERSION};

pub struct Migration {
    pub version: u32,
    pub description: String,
    pub up: fn(&Connection) -> Result<()>,
    pub down: fn(&Connection) -> Result<()>,
}

pub struct MigrationManager {
    migrations: HashMap<u32, Migration>,
}

impl MigrationManager {
    pub fn new() -> Self {
        let mut manager = Self {
            migrations: HashMap::new(),
        };
        manager.register_migrations();
        manager
    }

    fn register_migrations(&mut self) {
        // Initial schema migration
        self.add_migration(Migration {
            version: 1,
            description: "Initial schema creation".to_string(),
            up: |conn| {
                create_schema(conn)?;
                Ok(())
            },
            down: |conn| {
                super::schema::drop_schema(conn)?;
                Ok(())
            },
        });

        // Migration 2: Add retrospection tables
        self.add_migration(Migration {
            version: 2,
            description: "Add retrospection analysis tables".to_string(),
            up: |conn| {
                // Retrospection analyses storage
                conn.execute(
                    "CREATE TABLE retrospection_analyses (
                        id TEXT PRIMARY KEY,
                        session_id TEXT NOT NULL,
                        prompt_template_id TEXT NOT NULL,
                        analysis_content TEXT NOT NULL,
                        llm_service TEXT NOT NULL,
                        prompt_tokens INTEGER NOT NULL,
                        completion_tokens INTEGER NOT NULL,
                        total_tokens INTEGER NOT NULL,
                        estimated_cost REAL NOT NULL,
                        execution_time_ms INTEGER NOT NULL,
                        api_response_metadata TEXT,
                        status TEXT NOT NULL DEFAULT 'completed',
                        created_at TEXT NOT NULL,
                        updated_at TEXT NOT NULL,
                        FOREIGN KEY (session_id) REFERENCES chat_sessions (id)
                    )",
                    [],
                )?;

                // Prompt templates storage
                conn.execute(
                    "CREATE TABLE prompt_templates (
                        id TEXT PRIMARY KEY,
                        name TEXT NOT NULL,
                        description TEXT NOT NULL,
                        template TEXT NOT NULL,
                        category TEXT NOT NULL,
                        is_default BOOLEAN NOT NULL DEFAULT 0,
                        created_at TEXT NOT NULL,
                        modified_at TEXT NOT NULL,
                        UNIQUE(name)
                    )",
                    [],
                )?;

                // Template variables (normalized)
                conn.execute(
                    "CREATE TABLE prompt_variables (
                        template_id TEXT NOT NULL,
                        name TEXT NOT NULL,
                        description TEXT NOT NULL,
                        required BOOLEAN NOT NULL,
                        default_value TEXT,
                        PRIMARY KEY (template_id, name),
                        FOREIGN KEY (template_id) REFERENCES prompt_templates (id) ON DELETE CASCADE
                    )",
                    [],
                )?;

                // Analysis request queue
                conn.execute(
                    "CREATE TABLE analysis_requests (
                        id TEXT PRIMARY KEY,
                        session_id TEXT NOT NULL,
                        prompt_template_id TEXT NOT NULL,
                        template_variables TEXT NOT NULL, -- JSON
                        status TEXT NOT NULL DEFAULT 'queued',
                        error_message TEXT,
                        created_at TEXT NOT NULL,
                        started_at TEXT,
                        completed_at TEXT,
                        FOREIGN KEY (session_id) REFERENCES chat_sessions (id),
                        FOREIGN KEY (prompt_template_id) REFERENCES prompt_templates (id)
                    )",
                    [],
                )?;

                // Create indexes for better performance
                conn.execute(
                    "CREATE INDEX idx_retrospection_session ON retrospection_analyses (session_id)",
                    [],
                )?;
                conn.execute(
                    "CREATE INDEX idx_retrospection_created ON retrospection_analyses (created_at)",
                    [],
                )?;
                conn.execute(
                    "CREATE INDEX idx_requests_status ON analysis_requests (status)",
                    [],
                )?;
                conn.execute(
                    "CREATE INDEX idx_requests_created ON analysis_requests (created_at)",
                    [],
                )?;

                Ok(())
            },
            down: |conn| {
                conn.execute("DROP INDEX IF EXISTS idx_requests_created", [])?;
                conn.execute("DROP INDEX IF EXISTS idx_requests_status", [])?;
                conn.execute("DROP INDEX IF EXISTS idx_retrospection_created", [])?;
                conn.execute("DROP INDEX IF EXISTS idx_retrospection_session", [])?;
                conn.execute("DROP TABLE IF EXISTS analysis_requests", [])?;
                conn.execute("DROP TABLE IF EXISTS prompt_variables", [])?;
                conn.execute("DROP TABLE IF EXISTS prompt_templates", [])?;
                conn.execute("DROP TABLE IF EXISTS retrospection_analyses", [])?;
                Ok(())
            },
        });
    }

    pub fn add_migration(&mut self, migration: Migration) {
        self.migrations.insert(migration.version, migration);
    }

    pub fn get_current_version(&self, conn: &Connection) -> Result<u32> {
        // Check if schema_versions table exists
        let table_exists: bool = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='schema_versions'",
                [],
                |row| row.get::<_, i32>(0),
            )
            .unwrap_or(0)
            > 0;

        if !table_exists {
            return Ok(0);
        }

        // Get the highest version number
        let version: u32 = conn
            .query_row(
                "SELECT COALESCE(MAX(version), 0) FROM schema_versions",
                [],
                |row| row.get::<_, u32>(0),
            )
            .unwrap_or(0);

        Ok(version)
    }

    pub fn migrate_to_latest(&self, conn: &Connection) -> Result<()> {
        let current_version = self.get_current_version(conn)?;
        self.migrate_to_version(conn, SCHEMA_VERSION, current_version)
    }

    pub fn migrate_to_version(
        &self,
        conn: &Connection,
        target_version: u32,
        from_version: u32,
    ) -> Result<()> {
        if target_version == from_version {
            info!("Database is already at version {}", target_version);
            return Ok(());
        }

        if target_version > from_version {
            self.migrate_up(conn, from_version, target_version)
        } else {
            self.migrate_down(conn, from_version, target_version)
        }
    }

    fn migrate_up(&self, conn: &Connection, from_version: u32, to_version: u32) -> Result<()> {
        info!(
            "Migrating database from version {} to {}",
            from_version, to_version
        );

        let tx = conn.unchecked_transaction()?;

        for version in (from_version + 1)..=to_version {
            if let Some(migration) = self.migrations.get(&version) {
                info!("Applying migration {}: {}", version, migration.description);

                match (migration.up)(conn) {
                    Ok(()) => {
                        // Record successful migration (only if schema_versions table exists)
                        let table_exists: bool = conn.query_row(
                            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='schema_versions'",
                            [],
                            |row| row.get::<_, i32>(0)
                        ).unwrap_or(0) > 0;

                        if table_exists {
                            conn.execute(
                                "INSERT INTO schema_versions (version) VALUES (?1)",
                                [version],
                            )?;
                        }
                        info!("Successfully applied migration {}", version);
                    }
                    Err(e) => {
                        error!("Failed to apply migration {}: {}", version, e);
                        return Err(e);
                    }
                }
            } else {
                error!("Migration {} not found", version);
                return Err(rusqlite::Error::InvalidPath("Migration not found".into()));
            }
        }

        tx.commit()?;
        info!("Database migration completed successfully");
        Ok(())
    }

    fn migrate_down(&self, conn: &Connection, from_version: u32, to_version: u32) -> Result<()> {
        warn!(
            "Rolling back database from version {} to {}",
            from_version, to_version
        );

        let tx = conn.unchecked_transaction()?;

        for version in ((to_version + 1)..=from_version).rev() {
            if let Some(migration) = self.migrations.get(&version) {
                warn!(
                    "Rolling back migration {}: {}",
                    version, migration.description
                );

                // Remove migration record BEFORE calling down migration
                // Check if schema_versions table still exists
                let table_exists: bool = conn.query_row(
                    "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='schema_versions'",
                    [],
                    |row| row.get::<_, i32>(0)
                ).unwrap_or(0) > 0;

                if table_exists {
                    conn.execute("DELETE FROM schema_versions WHERE version = ?1", [version])?;
                }

                match (migration.down)(conn) {
                    Ok(()) => {
                        warn!("Successfully rolled back migration {}", version);
                    }
                    Err(e) => {
                        error!("Failed to roll back migration {}: {}", version, e);
                        return Err(e);
                    }
                }
            } else {
                error!("Migration {} not found for rollback", version);
                return Err(rusqlite::Error::InvalidPath("Migration not found".into()));
            }
        }

        tx.commit()?;
        warn!("Database rollback completed");
        Ok(())
    }

    pub fn get_migration_status(&self, conn: &Connection) -> Result<Vec<MigrationStatus>> {
        let current_version = self.get_current_version(conn)?;
        let mut status = Vec::new();

        let applied_versions: Vec<u32> = if current_version > 0 {
            let mut stmt = conn.prepare("SELECT version FROM schema_versions ORDER BY version")?;
            let rows = stmt
                .query_map([], |row| row.get::<_, u32>(0))?
                .collect::<Result<Vec<_>>>()?;
            rows
        } else {
            Vec::new()
        };

        for version in 1..=SCHEMA_VERSION {
            if let Some(migration) = self.migrations.get(&version) {
                let is_applied = applied_versions.contains(&version);
                status.push(MigrationStatus {
                    version,
                    description: migration.description.clone(),
                    applied: is_applied,
                });
            }
        }

        Ok(status)
    }

    pub fn reset_database(&self, conn: &Connection) -> Result<()> {
        warn!("Resetting database - all data will be lost!");

        // Drop all tables
        super::schema::drop_schema(conn)?;

        // Recreate from scratch
        self.migrate_to_latest(conn)?;

        info!("Database reset completed");
        Ok(())
    }

    pub fn validate_database(&self, conn: &Connection) -> Result<bool> {
        let current_version = self.get_current_version(conn)?;

        if current_version != SCHEMA_VERSION {
            warn!(
                "Database version mismatch: expected {}, found {}",
                SCHEMA_VERSION, current_version
            );
            return Ok(false);
        }

        // Verify all expected tables exist
        let expected_tables = vec![
            "chat_sessions",
            "messages",
            "projects",
            "usage_analyses",
            "llm_providers",
            "messages_fts",
            "retrospection_analyses",
            "prompt_templates",
            "prompt_variables",
            "analysis_requests",
        ];

        for table in expected_tables {
            let exists: bool = conn.query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?1",
                [table],
                |row| row.get::<_, i32>(0),
            )? > 0;

            if !exists {
                error!("Expected table '{}' not found", table);
                return Ok(false);
            }
        }

        info!("Database validation passed");
        Ok(true)
    }
}

#[derive(Debug)]
pub struct MigrationStatus {
    pub version: u32,
    pub description: String,
    pub applied: bool,
}

impl Default for MigrationManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    #[test]
    fn test_migration_manager() {
        let conn = Connection::open_in_memory().unwrap();
        let manager = MigrationManager::new();

        // Initial version should be 0
        assert_eq!(manager.get_current_version(&conn).unwrap(), 0);

        // Migrate to latest
        manager.migrate_to_latest(&conn).unwrap();
        assert_eq!(manager.get_current_version(&conn).unwrap(), SCHEMA_VERSION);

        // Verify database structure
        assert!(manager.validate_database(&conn).unwrap());
    }

    #[test]
    fn test_migration_status() {
        let conn = Connection::open_in_memory().unwrap();
        let manager = MigrationManager::new();

        // Before migration
        let status = manager.get_migration_status(&conn).unwrap();
        assert!(!status.is_empty());
        assert!(!status[0].applied);

        // After migration
        manager.migrate_to_latest(&conn).unwrap();
        let status = manager.get_migration_status(&conn).unwrap();
        assert!(status[0].applied);
    }

    #[test]
    fn test_database_reset() {
        let conn = Connection::open_in_memory().unwrap();
        let manager = MigrationManager::new();

        // Migrate and add some data
        manager.migrate_to_latest(&conn).unwrap();
        conn.execute(
            "INSERT INTO projects (id, name) VALUES ('test', 'Test Project')",
            [],
        )
        .unwrap();

        // Reset database
        manager.reset_database(&conn).unwrap();

        // Verify structure exists but data is gone
        assert!(manager.validate_database(&conn).unwrap());
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM projects", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn test_rollback() {
        let conn = Connection::open_in_memory().unwrap();
        let manager = MigrationManager::new();

        // Migrate to latest
        manager.migrate_to_latest(&conn).unwrap();
        assert_eq!(manager.get_current_version(&conn).unwrap(), SCHEMA_VERSION);

        // Rollback to version 0
        manager
            .migrate_to_version(&conn, 0, SCHEMA_VERSION)
            .unwrap();
        assert_eq!(manager.get_current_version(&conn).unwrap(), 0);

        // Verify tables are gone
        let table_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='chat_sessions'",
                [],
                |row| row.get::<_, i64>(0),
            )
            .unwrap();
        assert_eq!(table_count, 0);
    }
}
