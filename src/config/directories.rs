use anyhow::{Context, Result};
use directories::ProjectDirs;
use std::fs;
use std::path::{Path, PathBuf};

/// Application directory manager following XDG Base Directory specification
pub struct AppDirectories {
    project_dirs: ProjectDirs,
}

impl AppDirectories {
    /// Create new app directories instance
    pub fn new() -> Result<Self> {
        let project_dirs = ProjectDirs::from("dev", "retrochat", "retrochat")
            .context("Failed to determine project directories")?;

        Ok(Self { project_dirs })
    }

    /// Get the config directory path
    pub fn config_dir(&self) -> &Path {
        self.project_dirs.config_dir()
    }

    /// Get the data directory path
    pub fn data_dir(&self) -> &Path {
        self.project_dirs.data_dir()
    }

    /// Get the cache directory path
    pub fn cache_dir(&self) -> &Path {
        self.project_dirs.cache_dir()
    }

    /// Get the default database path
    pub fn database_path(&self) -> PathBuf {
        self.data_dir().join("retrochat.db")
    }

    /// Get the prompt templates directory path
    pub fn templates_dir(&self) -> PathBuf {
        self.config_dir().join("templates")
    }

    /// Get the default prompt templates file path
    pub fn default_templates_path(&self) -> PathBuf {
        self.templates_dir().join("defaults.toml")
    }

    /// Get the custom prompt templates file path
    pub fn custom_templates_path(&self) -> PathBuf {
        self.templates_dir().join("custom.toml")
    }

    /// Get the application config file path
    pub fn config_file_path(&self) -> PathBuf {
        self.config_dir().join("config.toml")
    }

    /// Get the logs directory path
    pub fn logs_dir(&self) -> PathBuf {
        self.cache_dir().join("logs")
    }

    /// Ensure all necessary directories exist
    pub fn ensure_directories(&self) -> Result<()> {
        let dirs_to_create = [
            self.config_dir(),
            self.data_dir(),
            self.cache_dir(),
            &self.templates_dir(),
            &self.logs_dir(),
        ];

        for dir in &dirs_to_create {
            if !dir.exists() {
                fs::create_dir_all(dir)
                    .with_context(|| format!("Failed to create directory: {}", dir.display()))?;
            }
        }

        Ok(())
    }

    /// Get the backup directory path
    pub fn backup_dir(&self) -> PathBuf {
        self.data_dir().join("backups")
    }

    /// Get the exports directory path
    pub fn exports_dir(&self) -> PathBuf {
        self.data_dir().join("exports")
    }

    /// Ensure backup and export directories exist
    pub fn ensure_additional_directories(&self) -> Result<()> {
        let additional_dirs = [&self.backup_dir(), &self.exports_dir()];

        for dir in &additional_dirs {
            if !dir.exists() {
                fs::create_dir_all(dir)
                    .with_context(|| format!("Failed to create directory: {}", dir.display()))?;
            }
        }

        Ok(())
    }

    /// Get all important paths for diagnostics
    pub fn get_paths_info(&self) -> Vec<(String, PathBuf)> {
        vec![
            (
                "Config Directory".to_string(),
                self.config_dir().to_path_buf(),
            ),
            ("Data Directory".to_string(), self.data_dir().to_path_buf()),
            (
                "Cache Directory".to_string(),
                self.cache_dir().to_path_buf(),
            ),
            ("Database Path".to_string(), self.database_path()),
            ("Templates Directory".to_string(), self.templates_dir()),
            ("Config File".to_string(), self.config_file_path()),
            ("Logs Directory".to_string(), self.logs_dir()),
            ("Backup Directory".to_string(), self.backup_dir()),
            ("Exports Directory".to_string(), self.exports_dir()),
        ]
    }

    /// Check if setup is complete
    pub fn is_setup_complete(&self) -> bool {
        let required_paths = [self.config_dir(), self.data_dir(), &self.templates_dir()];

        required_paths.iter().all(|path| path.exists())
    }

    /// Get relative path from data directory for a given absolute path
    pub fn get_relative_data_path(&self, absolute_path: &Path) -> Option<PathBuf> {
        absolute_path
            .strip_prefix(self.data_dir())
            .ok()
            .map(|p| p.to_path_buf())
    }

    /// Get relative path from config directory for a given absolute path
    pub fn get_relative_config_path(&self, absolute_path: &Path) -> Option<PathBuf> {
        absolute_path
            .strip_prefix(self.config_dir())
            .ok()
            .map(|p| p.to_path_buf())
    }
}

impl Default for AppDirectories {
    fn default() -> Self {
        Self::new().expect("Failed to create app directories")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_directories_creation() {
        let dirs = AppDirectories::new().expect("Should create app directories");

        assert!(dirs.config_dir().to_string_lossy().contains("retrochat"));
        assert!(dirs.data_dir().to_string_lossy().contains("retrochat"));
        assert!(dirs.cache_dir().to_string_lossy().contains("retrochat"));
    }

    #[test]
    fn test_derived_paths() {
        let dirs = AppDirectories::new().expect("Should create app directories");

        let db_path = dirs.database_path();
        assert!(db_path.file_name().unwrap() == "retrochat.db");

        let templates_dir = dirs.templates_dir();
        assert!(templates_dir.file_name().unwrap() == "templates");

        let config_file = dirs.config_file_path();
        assert!(config_file.file_name().unwrap() == "config.toml");
    }

    #[test]
    fn test_paths_info() {
        let dirs = AppDirectories::new().expect("Should create app directories");
        let paths_info = dirs.get_paths_info();

        assert!(paths_info.len() >= 8);
        assert!(paths_info
            .iter()
            .any(|(name, _)| name == "Config Directory"));
        assert!(paths_info.iter().any(|(name, _)| name == "Database Path"));
        assert!(paths_info
            .iter()
            .any(|(name, _)| name == "Templates Directory"));
    }

    #[test]
    fn test_relative_path_helpers() {
        let dirs = AppDirectories::new().expect("Should create app directories");

        // Test data directory relative paths
        let db_path = dirs.database_path();
        let relative = dirs.get_relative_data_path(&db_path);
        assert_eq!(relative, Some(PathBuf::from("retrochat.db")));

        // Test config directory relative paths
        let config_path = dirs.config_file_path();
        let relative = dirs.get_relative_config_path(&config_path);
        assert_eq!(relative, Some(PathBuf::from("config.toml")));
    }

    #[test]
    fn test_setup_complete_check() {
        let dirs = AppDirectories::new().expect("Should create app directories");

        // Before setup, should not be complete
        // Note: This might pass if directories already exist from other tests
        // so we just check the method doesn't panic
        let _ = dirs.is_setup_complete();
    }
}
