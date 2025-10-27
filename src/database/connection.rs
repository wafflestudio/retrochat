use anyhow::{Context, Result as AnyhowResult};
use sqlx::{sqlite::SqlitePool, Pool, Sqlite};
use std::path::{Path, PathBuf};
use tracing::{debug, info};

/// Get the default database path in the user's home directory
pub fn get_default_db_path() -> AnyhowResult<PathBuf> {
    let home_dir = dirs::home_dir().context("Could not find home directory")?;
    Ok(home_dir.join(".retrochat").join("retrochat.db"))
}

#[derive(Clone)]
pub struct DatabaseManager {
    db_path: PathBuf,
    pool: Pool<Sqlite>,
}

impl DatabaseManager {
    pub async fn new(db_path: impl AsRef<Path>) -> AnyhowResult<Self> {
        let db_path = db_path.as_ref().to_path_buf();

        // Ensure parent directory exists
        if let Some(parent) = db_path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent).with_context(|| {
                    format!("Failed to create database directory: {}", parent.display())
                })?;
            }
        }

        // Ensure the database file can be created/opened
        if !db_path.exists() {
            std::fs::File::create(&db_path).with_context(|| {
                format!("Failed to create database file: {}", db_path.display())
            })?;
        }

        // Check for potentially corrupted WAL files before connecting
        Self::check_and_cleanup_wal_files(&db_path)?;

        // Create SQLite connection string
        let database_url = format!("sqlite://{}", db_path.display());

        // Create connection pool with optimized settings
        let pool = SqlitePool::connect(&database_url)
            .await
            .with_context(|| format!("Failed to connect to database at: {}", db_path.display()))?;

        let manager = Self { db_path, pool };

        // Optimize database for performance
        manager.optimize_for_performance().await?;

        // Run migrations
        manager.run_migrations().await?;

        info!(
            "SQLx database initialized at: {}",
            manager.db_path.display()
        );
        Ok(manager)
    }

    /// Check for and cleanup potentially corrupted WAL files
    fn check_and_cleanup_wal_files(db_path: &Path) -> AnyhowResult<()> {
        let wal_path = db_path.with_extension("db-wal");
        let shm_path = db_path.with_extension("db-shm");

        // Only attempt cleanup if both files exist and are very small (likely corrupted)
        // Normal WAL files can be large, so we use a heuristic here
        let should_cleanup = if wal_path.exists() && shm_path.exists() {
            match (std::fs::metadata(&wal_path), std::fs::metadata(&shm_path)) {
                (Ok(wal_meta), Ok(shm_meta)) => {
                    // If WAL file is suspiciously small (< 100 bytes) or SHM is wrong size
                    // (should be 32KB for SQLite), these might be corrupted
                    wal_meta.len() < 100 || shm_meta.len() != 32768
                }
                _ => false,
            }
        } else {
            false
        };

        if should_cleanup {
            tracing::warn!(
                "Detected potentially corrupted WAL files, removing them before connection"
            );

            // Best-effort cleanup - don't fail if files don't exist or can't be removed
            if wal_path.exists() {
                match std::fs::remove_file(&wal_path) {
                    Ok(_) => tracing::debug!("Removed WAL file: {}", wal_path.display()),
                    Err(e) => {
                        tracing::warn!("Failed to remove WAL file {}: {}", wal_path.display(), e)
                    }
                }
            }

            if shm_path.exists() {
                match std::fs::remove_file(&shm_path) {
                    Ok(_) => tracing::debug!("Removed SHM file: {}", shm_path.display()),
                    Err(e) => {
                        tracing::warn!("Failed to remove SHM file {}: {}", shm_path.display(), e)
                    }
                }
            }
        }

        Ok(())
    }

    pub async fn open_in_memory() -> AnyhowResult<Self> {
        let pool = SqlitePool::connect("sqlite::memory:")
            .await
            .context("Failed to create in-memory database")?;

        let manager = Self {
            db_path: PathBuf::from(":memory:"),
            pool,
        };

        // Run migrations
        manager.run_migrations().await?;

        debug!("SQLx in-memory database initialized");
        Ok(manager)
    }

    async fn optimize_for_performance(&self) -> AnyhowResult<()> {
        // Enable WAL mode for better concurrency
        sqlx::query("PRAGMA journal_mode = WAL")
            .execute(&self.pool)
            .await
            .context("Failed to set WAL mode")?;

        // Increase cache size to 64MB for better performance
        sqlx::query("PRAGMA cache_size = -64000")
            .execute(&self.pool)
            .await
            .context("Failed to set cache size")?;

        // Use memory for temp store
        sqlx::query("PRAGMA temp_store = MEMORY")
            .execute(&self.pool)
            .await
            .context("Failed to set temp store")?;

        // Optimize synchronous mode for better write performance
        // NORMAL is safe with WAL mode and much faster than FULL
        sqlx::query("PRAGMA synchronous = NORMAL")
            .execute(&self.pool)
            .await
            .context("Failed to set synchronous mode")?;

        debug!("Database optimized for performance");
        Ok(())
    }

    async fn run_migrations(&self) -> AnyhowResult<()> {
        // Run SQLx migrations
        sqlx::migrate!("./migrations")
            .run(&self.pool)
            .await
            .context("Failed to run database migrations")?;

        info!("Database migrations completed successfully");
        Ok(())
    }

    pub fn pool(&self) -> &Pool<Sqlite> {
        &self.pool
    }

    pub async fn close(self) -> AnyhowResult<()> {
        self.pool.close().await;
        Ok(())
    }

    pub async fn health_check(&self) -> AnyhowResult<()> {
        sqlx::query("SELECT 1")
            .fetch_one(&self.pool)
            .await
            .context("Database health check failed")?;
        Ok(())
    }
}

impl Drop for DatabaseManager {
    fn drop(&mut self) {
        // SQLx pool will be closed automatically when dropped
        debug!("SQLx database manager dropped");
    }
}
