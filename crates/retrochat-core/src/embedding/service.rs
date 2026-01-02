//! Embedding service for generating text embeddings.

use anyhow::{Context, Result};
use fastembed::{EmbeddingModel as FastEmbedModel, InitOptions, TextEmbedding};

use super::config::EmbeddingConfig;
use super::models::{EmbeddingModel, ModelInfo};

/// Service for generating text embeddings using local models.
///
/// This service wraps FastEmbed-rs to provide text embedding generation
/// without requiring external API calls. Models are downloaded on first use
/// and cached locally.
pub struct EmbeddingService {
    model: TextEmbedding,
    info: ModelInfo,
}

impl EmbeddingService {
    /// Create a new embedding service with the given configuration.
    ///
    /// This will download the model on first use if not already cached.
    pub fn new(config: EmbeddingConfig) -> Result<Self> {
        let fastembed_model = Self::to_fastembed_model(&config.model);

        let init_options = InitOptions::new(fastembed_model)
            .with_cache_dir(config.get_cache_dir())
            .with_show_download_progress(config.show_download_progress);

        let model =
            TextEmbedding::try_new(init_options).context("Failed to initialize embedding model")?;

        let info = ModelInfo::from(config.model);

        Ok(Self { model, info })
    }

    /// Create a new embedding service with default configuration.
    pub fn with_defaults() -> Result<Self> {
        Self::new(EmbeddingConfig::default())
    }

    /// Get information about the loaded model.
    pub fn model_info(&self) -> &ModelInfo {
        &self.info
    }

    /// Generate an embedding for a single text.
    ///
    /// Returns a vector of f32 values representing the text's semantic embedding.
    pub fn embed_text(&self, text: &str) -> Result<Vec<f32>> {
        let embeddings = self
            .model
            .embed(vec![text], None)
            .context("Failed to generate embedding")?;

        embeddings
            .into_iter()
            .next()
            .ok_or_else(|| anyhow::anyhow!("No embedding returned"))
    }

    /// Generate embeddings for multiple texts in a batch.
    ///
    /// This is more efficient than calling `embed_text` multiple times
    /// as it processes all texts in a single batch.
    pub fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>> {
        if texts.is_empty() {
            return Ok(Vec::new());
        }

        let texts_vec: Vec<String> = texts.iter().map(|s| s.to_string()).collect();
        let text_refs: Vec<&str> = texts_vec.iter().map(|s| s.as_str()).collect();

        self.model
            .embed(text_refs, None)
            .context("Failed to generate batch embeddings")
    }

    /// Get the number of dimensions in the embeddings.
    pub fn dimensions(&self) -> usize {
        self.info.dimensions
    }

    /// Convert our model enum to FastEmbed's model enum.
    fn to_fastembed_model(model: &EmbeddingModel) -> FastEmbedModel {
        match model {
            EmbeddingModel::BGESmallENV15 => FastEmbedModel::BGESmallENV15,
            EmbeddingModel::BGESmallENV15Q => FastEmbedModel::BGESmallENV15,
            EmbeddingModel::AllMiniLML6V2 => FastEmbedModel::AllMiniLML6V2,
            EmbeddingModel::AllMiniLML6V2Q => FastEmbedModel::AllMiniLML6V2,
            EmbeddingModel::BGEBaseENV15 => FastEmbedModel::BGEBaseENV15,
            EmbeddingModel::BGEBaseENV15Q => FastEmbedModel::BGEBaseENV15,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    // Note: These tests require model download and may be slow on first run.
    // They are marked as #[ignore] by default for CI.

    fn test_config() -> EmbeddingConfig {
        EmbeddingConfig::new(EmbeddingModel::AllMiniLML6V2)
            .with_cache_dir(PathBuf::from("/tmp/retrochat-test-models"))
            .with_show_download_progress(false)
    }

    #[test]
    #[ignore = "Requires model download"]
    fn test_embed_text() {
        let service = EmbeddingService::new(test_config()).unwrap();

        let embedding = service.embed_text("Hello, world!").unwrap();

        assert_eq!(embedding.len(), 384);
        // Embeddings should be normalized (approximately unit length)
        let magnitude: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((magnitude - 1.0).abs() < 0.1);
    }

    #[test]
    #[ignore = "Requires model download"]
    fn test_embed_batch() {
        let service = EmbeddingService::new(test_config()).unwrap();

        let texts = vec!["First text", "Second text", "Third text"];
        let embeddings = service.embed_batch(&texts).unwrap();

        assert_eq!(embeddings.len(), 3);
        for embedding in &embeddings {
            assert_eq!(embedding.len(), 384);
        }
    }

    #[test]
    #[ignore = "Requires model download"]
    fn test_embed_batch_empty() {
        let service = EmbeddingService::new(test_config()).unwrap();

        let texts: Vec<&str> = vec![];
        let embeddings = service.embed_batch(&texts).unwrap();

        assert!(embeddings.is_empty());
    }

    #[test]
    #[ignore = "Requires model download"]
    fn test_similar_texts_have_similar_embeddings() {
        let service = EmbeddingService::new(test_config()).unwrap();

        let text1 = "The quick brown fox jumps over the lazy dog";
        let text2 = "A fast brown fox leaps over a sleepy dog";
        let text3 = "Machine learning is a subset of artificial intelligence";

        let emb1 = service.embed_text(text1).unwrap();
        let emb2 = service.embed_text(text2).unwrap();
        let emb3 = service.embed_text(text3).unwrap();

        // Cosine similarity
        let sim_12: f32 = emb1.iter().zip(&emb2).map(|(a, b)| a * b).sum();
        let sim_13: f32 = emb1.iter().zip(&emb3).map(|(a, b)| a * b).sum();

        // Similar texts should have higher similarity
        assert!(
            sim_12 > sim_13,
            "Similar texts should have higher cosine similarity"
        );
    }

    #[test]
    fn test_model_info() {
        // This test doesn't require model download
        let info = ModelInfo::from(EmbeddingModel::BGESmallENV15);

        assert_eq!(info.dimensions, 384);
        assert!(!info.quantized);
        assert_eq!(info.name, "BGESmallENV15");
    }
}
