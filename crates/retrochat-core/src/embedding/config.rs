//! Configuration for embedding generation.

use std::path::PathBuf;

use super::models::EmbeddingModel;

/// Configuration for the embedding service.
#[derive(Debug, Clone)]
pub struct EmbeddingConfig {
    /// The embedding model to use.
    pub model: EmbeddingModel,

    /// Directory to cache downloaded models.
    /// Defaults to `~/.retrochat/models/` if not specified.
    pub cache_dir: Option<PathBuf>,

    /// Whether to show download progress when fetching models.
    pub show_download_progress: bool,
}

impl Default for EmbeddingConfig {
    fn default() -> Self {
        Self {
            model: EmbeddingModel::BGESmallENV15,
            cache_dir: None,
            show_download_progress: true,
        }
    }
}

impl EmbeddingConfig {
    /// Create a new configuration with the specified model.
    pub fn new(model: EmbeddingModel) -> Self {
        Self {
            model,
            ..Default::default()
        }
    }

    /// Set the cache directory for downloaded models.
    pub fn with_cache_dir(mut self, path: PathBuf) -> Self {
        self.cache_dir = Some(path);
        self
    }

    /// Set whether to show download progress.
    pub fn with_show_download_progress(mut self, show: bool) -> Self {
        self.show_download_progress = show;
        self
    }

    /// Get the cache directory, using default if not specified.
    pub fn get_cache_dir(&self) -> PathBuf {
        self.cache_dir.clone().unwrap_or_else(|| {
            dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join(".retrochat")
                .join("models")
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = EmbeddingConfig::default();
        assert!(matches!(config.model, EmbeddingModel::BGESmallENV15));
        assert!(config.cache_dir.is_none());
        assert!(config.show_download_progress);
    }

    #[test]
    fn test_config_builder() {
        let config = EmbeddingConfig::new(EmbeddingModel::AllMiniLML6V2)
            .with_cache_dir(PathBuf::from("/tmp/models"))
            .with_show_download_progress(false);

        assert!(matches!(config.model, EmbeddingModel::AllMiniLML6V2));
        assert_eq!(config.cache_dir, Some(PathBuf::from("/tmp/models")));
        assert!(!config.show_download_progress);
    }

    #[test]
    fn test_get_cache_dir_default() {
        let config = EmbeddingConfig::default();
        let cache_dir = config.get_cache_dir();
        assert!(cache_dir.to_string_lossy().contains(".retrochat"));
        assert!(cache_dir.to_string_lossy().contains("models"));
    }

    #[test]
    fn test_get_cache_dir_custom() {
        let config = EmbeddingConfig::default().with_cache_dir(PathBuf::from("/custom/path"));
        assert_eq!(config.get_cache_dir(), PathBuf::from("/custom/path"));
    }
}
