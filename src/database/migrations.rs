use anyhow::{Context, Result as AnyhowResult};
use sqlx::{Pool, Sqlite};
use tracing::{error, info, warn};

pub struct MigrationManager {
    pool: Pool<Sqlite>,
}

impl MigrationManager {
    pub fn new(pool: Pool<Sqlite>) -> Self {
        Self { pool }
    }

    pub async fn get_current_version(&self) -> AnyhowResult<u32> {
        // Check if schema_versions table exists
        let count: i32 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='schema_versions'",
        )
        .fetch_one(&self.pool)
        .await
        .unwrap_or(0);

        let table_exists = count > 0;

        if !table_exists {
            return Ok(0);
        }

        // Get the highest version number
        let version: u32 =
            sqlx::query_scalar("SELECT COALESCE(MAX(version), 0) FROM schema_versions")
                .fetch_one(&self.pool)
                .await
                .unwrap_or(0);

        Ok(version)
    }

    pub async fn migrate_to_latest(&self) -> AnyhowResult<()> {
        let current_version = self.get_current_version().await?;
        let target_version = self.get_latest_version().await?;

        self.migrate_to_version(target_version, current_version)
            .await
    }

    pub async fn migrate_to_version(
        &self,
        target_version: u32,
        from_version: u32,
    ) -> AnyhowResult<()> {
        if target_version == from_version {
            info!("Database is already at version {}", target_version);
            return Ok(());
        }

        if target_version > from_version {
            self.migrate_up(from_version, target_version).await
        } else {
            self.migrate_down(from_version, target_version).await
        }
    }

    async fn migrate_up(&self, from_version: u32, to_version: u32) -> AnyhowResult<()> {
        info!(
            "Migrating database from version {} to {}",
            from_version, to_version
        );

        let mut tx = self.pool.begin().await?;

        for version in (from_version + 1)..=to_version {
            info!("Applying migration {}", version);

            // SQLx migrations are handled by the migrate! macro
            // This is just for tracking and logging
            sqlx::query("INSERT INTO schema_versions (version) VALUES (?)")
                .bind(version)
                .execute(&mut *tx)
                .await
                .with_context(|| format!("Failed to record migration {version}"))?;

            info!("Successfully applied migration {}", version);
        }

        tx.commit().await?;
        info!("Database migration completed successfully");
        Ok(())
    }

    async fn migrate_down(&self, from_version: u32, to_version: u32) -> AnyhowResult<()> {
        warn!(
            "Rolling back database from version {} to {}",
            from_version, to_version
        );

        let mut tx = self.pool.begin().await?;

        for version in (to_version + 1..=from_version).rev() {
            warn!("Rolling back migration {}", version);

            // Remove migration record
            sqlx::query("DELETE FROM schema_versions WHERE version = ?")
                .bind(version)
                .execute(&mut *tx)
                .await
                .with_context(|| format!("Failed to remove migration record {version}"))?;

            warn!("Successfully rolled back migration {}", version);
        }

        tx.commit().await?;
        warn!("Database rollback completed");
        Ok(())
    }

    async fn get_latest_version(&self) -> AnyhowResult<u32> {
        // For now, we'll use a hardcoded value
        // In a real implementation, you might scan the migrations directory
        Ok(1)
    }

    pub async fn get_migration_status(&self) -> AnyhowResult<Vec<MigrationStatus>> {
        let current_version = self.get_current_version().await?;
        let mut status = Vec::new();

        let applied_versions: Vec<u32> = if current_version > 0 {
            sqlx::query_scalar("SELECT version FROM schema_versions ORDER BY version")
                .fetch_all(&self.pool)
                .await?
        } else {
            Vec::new()
        };

        // For now, we only have one migration
        let is_applied = applied_versions.contains(&1);
        status.push(MigrationStatus {
            version: 1,
            description: "Initial schema creation".to_string(),
            applied: is_applied,
        });

        Ok(status)
    }

    pub async fn reset_database(&self) -> AnyhowResult<()> {
        warn!("Resetting database - all data will be lost!");

        // Drop all tables
        self.drop_all_tables().await?;

        // Recreate from scratch
        self.migrate_to_latest().await?;

        info!("Database reset completed");
        Ok(())
    }

    async fn drop_all_tables(&self) -> AnyhowResult<()> {
        // Get all table names
        let tables: Vec<String> = sqlx::query_scalar(
            "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%'",
        )
        .fetch_all(&self.pool)
        .await?;

        // Drop each table
        for table in tables {
            sqlx::query(&format!("DROP TABLE IF EXISTS {table}"))
                .execute(&self.pool)
                .await?;
        }

        Ok(())
    }

    pub async fn validate_database(&self) -> AnyhowResult<bool> {
        // Check if all required tables exist
        let required_tables = [
            "schema_versions",
            "projects",
            "chat_sessions",
            "messages",
            "usage_analyses",
            "llm_providers",
            "messages_fts",
        ];

        for table in &required_tables {
            let count: i32 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name = ?",
            )
            .bind(table)
            .fetch_one(&self.pool)
            .await?;

            let exists = count > 0;

            if !exists {
                error!("Required table {} does not exist", table);
                return Ok(false);
            }
        }

        info!("Database validation passed");
        Ok(true)
    }
}

#[derive(Debug, Clone)]
pub struct MigrationStatus {
    pub version: u32,
    pub description: String,
    pub applied: bool,
}
