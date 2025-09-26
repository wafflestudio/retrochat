use anyhow::{Context, Result as AnyhowResult};
use rusqlite::{Connection, OpenFlags, Result};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tracing::{debug, error, info};

use super::migrations::MigrationManager;

#[derive(Debug)]
pub struct DatabaseManager {
    db_path: PathBuf,
    connection: Arc<Mutex<Connection>>,
}

impl DatabaseManager {
    pub fn new(db_path: impl AsRef<Path>) -> AnyhowResult<Self> {
        let db_path = db_path.as_ref().to_path_buf();

        // Ensure parent directory exists
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent).with_context(|| {
                format!("Failed to create database directory: {}", parent.display())
            })?;
        }

        // Open connection with proper flags
        let connection = Connection::open_with_flags(
            &db_path,
            OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_CREATE,
        )
        .with_context(|| format!("Failed to open database at: {}", db_path.display()))?;

        // Configure SQLite settings for performance and safety
        connection.execute("PRAGMA foreign_keys = ON", [])?;
        // Some PRAGMA statements return values, so we need to consume them
        connection
            .prepare("PRAGMA journal_mode = WAL")?
            .query_map([], |_| Ok(()))?
            .for_each(drop);
        connection
            .prepare("PRAGMA synchronous = NORMAL")?
            .query_map([], |_| Ok(()))?
            .for_each(drop);
        connection
            .prepare("PRAGMA cache_size = -64000")?
            .query_map([], |_| Ok(()))?
            .for_each(drop); // 64MB cache
        connection
            .prepare("PRAGMA temp_store = memory")?
            .query_map([], |_| Ok(()))?
            .for_each(drop);
        connection
            .prepare("PRAGMA mmap_size = 268435456")?
            .query_map([], |_| Ok(()))?
            .for_each(drop); // 256MB mmap

        let manager = Self {
            db_path,
            connection: Arc::new(Mutex::new(connection)),
        };

        // Run migrations
        manager.run_migrations()?;

        info!("Database initialized at: {}", manager.db_path.display());
        Ok(manager)
    }

    pub fn open_in_memory() -> AnyhowResult<Self> {
        let connection =
            Connection::open_in_memory().context("Failed to create in-memory database")?;

        // Configure in-memory settings
        connection.execute("PRAGMA foreign_keys = ON", [])?;
        connection
            .prepare("PRAGMA cache_size = -32000")?
            .query_map([], |_| Ok(()))?
            .for_each(drop); // 32MB cache for in-memory

        let manager = Self {
            db_path: PathBuf::from(":memory:"),
            connection: Arc::new(Mutex::new(connection)),
        };

        // Run migrations
        manager.run_migrations()?;

        debug!("In-memory database initialized");
        Ok(manager)
    }

    fn run_migrations(&self) -> AnyhowResult<()> {
        let migration_manager = MigrationManager::new();
        let conn = self.connection.lock().unwrap();

        migration_manager
            .migrate_to_latest(&conn)
            .context("Failed to run database migrations")?;

        if !migration_manager.validate_database(&conn)? {
            return Err(anyhow::anyhow!(
                "Database validation failed after migration"
            ));
        }

        Ok(())
    }

    pub fn with_connection<F, R>(&self, f: F) -> AnyhowResult<R>
    where
        F: FnOnce(&Connection) -> Result<R>,
    {
        let conn = self.connection.lock().unwrap();
        f(&conn).with_context(|| "Database operation failed")
    }

    pub fn with_connection_anyhow<F, R>(&self, f: F) -> AnyhowResult<R>
    where
        F: FnOnce(&Connection) -> AnyhowResult<R>,
    {
        let conn = self.connection.lock().unwrap();
        f(&conn)
    }

    pub fn with_transaction<F, R>(&self, f: F) -> AnyhowResult<R>
    where
        F: FnOnce(&Connection) -> Result<R>,
    {
        let conn = self.connection.lock().unwrap();
        let tx = conn
            .unchecked_transaction()
            .context("Failed to start transaction")?;

        let result = f(&conn);

        match result {
            Ok(value) => {
                tx.commit().context("Failed to commit transaction")?;
                Ok(value)
            }
            Err(e) => {
                if let Err(rollback_err) = tx.rollback() {
                    error!("Failed to rollback transaction: {}", rollback_err);
                }
                Err(anyhow::anyhow!("Transaction failed: {e}"))
            }
        }
    }

    pub fn backup_to_file(&self, backup_path: impl AsRef<Path>) -> AnyhowResult<()> {
        let backup_path = backup_path.as_ref();

        // Ensure parent directory exists
        if let Some(parent) = backup_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let conn = self.connection.lock().unwrap();

        // Use SQLite backup API
        let mut backup_conn = Connection::open(backup_path)?;
        let backup = rusqlite::backup::Backup::new(&conn, &mut backup_conn)?;
        backup.run_to_completion(5, std::time::Duration::from_millis(250), None)?;

        info!("Database backed up to: {}", backup_path.display());
        Ok(())
    }

    pub fn restore_from_file(&self, backup_path: impl AsRef<Path>) -> AnyhowResult<()> {
        let backup_path = backup_path.as_ref();

        if !backup_path.exists() {
            return Err(anyhow::anyhow!(
                "Backup file does not exist: {}",
                backup_path.display()
            ));
        }

        let backup_conn = Connection::open(backup_path)?;
        let mut conn = self.connection.lock().unwrap();

        // Restore from backup
        let backup = rusqlite::backup::Backup::new(&backup_conn, &mut conn)?;
        backup.run_to_completion(5, std::time::Duration::from_millis(250), None)?;

        info!("Database restored from: {}", backup_path.display());
        Ok(())
    }

    pub fn vacuum(&self) -> AnyhowResult<()> {
        let conn = self.connection.lock().unwrap();
        conn.execute("VACUUM", [])?;
        info!("Database vacuum completed");
        Ok(())
    }

    pub fn analyze(&self) -> AnyhowResult<()> {
        let conn = self.connection.lock().unwrap();
        conn.execute("ANALYZE", [])?;
        debug!("Database analyze completed");
        Ok(())
    }

    pub fn get_database_size(&self) -> AnyhowResult<u64> {
        if self.db_path.to_string_lossy() == ":memory:" {
            return Ok(0); // In-memory database size is not meaningful
        }

        let metadata = std::fs::metadata(&self.db_path)?;
        Ok(metadata.len())
    }

    pub fn get_table_info(&self) -> AnyhowResult<Vec<TableInfo>> {
        self.with_connection(|conn| {
            let mut stmt = conn.prepare(
                "SELECT name,
                        (SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND tbl_name=m.name) as index_count
                 FROM sqlite_master m
                 WHERE type='table' AND name NOT LIKE 'sqlite_%'
                 ORDER BY name"
            )?;

            let table_info = stmt.query_map([], |row| {
                let table_name: String = row.get(0)?;
                let index_count: u32 = row.get(1)?;

                // Get row count for this table
                let row_count: u64 = conn.query_row(
                    &format!("SELECT COUNT(*) FROM {table_name}"),
                    [],
                    |row| row.get(0)
                )?;

                Ok(TableInfo {
                    name: table_name,
                    row_count,
                    index_count,
                })
            })?.collect::<Result<Vec<_>>>()?;

            Ok(table_info)
        })
    }

    pub fn check_integrity(&self) -> AnyhowResult<bool> {
        self.with_connection(|conn| {
            let result: String = conn.query_row("PRAGMA integrity_check", [], |row| row.get(0))?;
            Ok(result == "ok")
        })
    }

    pub fn optimize(&self) -> AnyhowResult<()> {
        info!("Optimizing database...");

        self.with_connection(|conn| {
            // Update table statistics
            conn.execute("ANALYZE", [])?;

            // Optimize individual tables
            conn.execute("PRAGMA optimize", [])?;

            Ok(())
        })?;

        info!("Database optimization completed");
        Ok(())
    }

    pub fn get_db_path(&self) -> &Path {
        &self.db_path
    }
}

#[derive(Debug)]
pub struct TableInfo {
    pub name: String,
    pub row_count: u64,
    pub index_count: u32,
}

impl Clone for DatabaseManager {
    fn clone(&self) -> Self {
        Self {
            db_path: self.db_path.clone(),
            connection: Arc::clone(&self.connection),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_database_manager_file() {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");

        let manager = DatabaseManager::new(&db_path).unwrap();
        assert!(db_path.exists());
        assert!(manager.check_integrity().unwrap());
    }

    #[test]
    fn test_database_manager_memory() {
        let manager = DatabaseManager::open_in_memory().unwrap();
        assert!(manager.check_integrity().unwrap());
        assert_eq!(manager.get_database_size().unwrap(), 0);
    }

    #[test]
    fn test_transaction() {
        let manager = DatabaseManager::open_in_memory().unwrap();

        // Successful transaction
        let result = manager.with_transaction(|conn| {
            conn.execute(
                "INSERT INTO projects (id, name) VALUES ('test', 'Test Project')",
                [],
            )?;
            Ok(42)
        });
        assert_eq!(result.unwrap(), 42);

        // Verify data was committed
        let count = manager
            .with_connection(|conn| {
                conn.query_row("SELECT COUNT(*) FROM projects", [], |row| {
                    row.get::<_, i64>(0)
                })
            })
            .unwrap();
        assert_eq!(count, 1);

        // Failed transaction
        let result = manager.with_transaction(|conn| {
            conn.execute(
                "INSERT INTO projects (id, name) VALUES ('test2', 'Test Project 2')",
                [],
            )?;
            // Force an error
            conn.execute("INVALID SQL", [])?;
            Ok(())
        });
        assert!(result.is_err());

        // Verify rollback worked
        let count = manager
            .with_connection(|conn| {
                conn.query_row("SELECT COUNT(*) FROM projects", [], |row| {
                    row.get::<_, i64>(0)
                })
            })
            .unwrap();
        assert_eq!(count, 1); // Still just the first record
    }

    #[test]
    fn test_table_info() {
        let manager = DatabaseManager::open_in_memory().unwrap();
        let info = manager.get_table_info().unwrap();

        assert!(!info.is_empty());

        // Find the projects table
        let projects_table = info.iter().find(|t| t.name == "projects").unwrap();
        assert_eq!(projects_table.row_count, 0);
        assert!(projects_table.index_count > 0); // Should have at least the name index
    }

    #[test]
    fn test_backup_restore() {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let backup_path = temp_dir.path().join("backup.db");

        let manager = DatabaseManager::new(&db_path).unwrap();

        // Add some data
        manager
            .with_connection(|conn| {
                conn.execute(
                    "INSERT INTO projects (id, name) VALUES ('test', 'Test Project')",
                    [],
                )
            })
            .unwrap();

        // Backup
        manager.backup_to_file(&backup_path).unwrap();
        assert!(backup_path.exists());

        // Create new manager and restore
        let manager2 = DatabaseManager::new(temp_dir.path().join("test2.db")).unwrap();
        manager2.restore_from_file(&backup_path).unwrap();

        // Verify data was restored
        let count = manager2
            .with_connection(|conn| {
                conn.query_row("SELECT COUNT(*) FROM projects", [], |row| {
                    row.get::<_, i64>(0)
                })
            })
            .unwrap();
        assert_eq!(count, 1);
    }
}
