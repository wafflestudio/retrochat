//! Embedding service for generating text embeddings
//!
//! This service provides text embedding generation with support for:
//! - MLX-based embeddings on macOS (when RETROCHAT_USE_MLX is enabled)
//! - Dummy embeddings (768 dimensions) for development and testing
//!
//! Platform Support:
//! - macOS: Full MLX support when enabled
//! - Windows/Linux: Shows warning, falls back to dummy embeddings

use anyhow::Result;
use tracing::{info, warn};

/// Standard embedding dimension size (compatible with many embedding models)
pub const EMBEDDING_DIM: usize = 768;

/// Embedding service for generating text embeddings
pub struct EmbeddingService {
    enabled: bool,
    mlx_available: bool,
}

impl EmbeddingService {
    /// Create a new embedding service
    ///
    /// Checks platform support and environment configuration:
    /// - On macOS with RETROCHAT_USE_MLX=true: Enables MLX-based embeddings
    /// - On other platforms or when disabled: Uses dummy embeddings
    pub fn new() -> Self {
        let use_mlx_env = std::env::var(crate::env::embedding::USE_MLX)
            .unwrap_or_else(|_| "false".to_string())
            .to_lowercase();
        let use_mlx = use_mlx_env == "true" || use_mlx_env == "1";

        let mlx_available = Self::check_mlx_support();

        let enabled = if use_mlx {
            if mlx_available {
                info!("Embedding service enabled with MLX support");
                true
            } else {
                warn!(
                    "RETROCHAT_USE_MLX is enabled but MLX is not supported on this platform. \
                     MLX only works on macOS. Embedding service will use dummy embeddings."
                );
                false
            }
        } else {
            info!("Embedding service using dummy embeddings (RETROCHAT_USE_MLX not enabled)");
            false
        };

        Self {
            enabled,
            mlx_available,
        }
    }

    /// Check if MLX is supported on the current platform
    fn check_mlx_support() -> bool {
        #[cfg(target_os = "macos")]
        {
            // On macOS, MLX is available if the feature is enabled
            #[cfg(feature = "mlx")]
            {
                true
            }
            #[cfg(not(feature = "mlx"))]
            {
                false
            }
        }
        #[cfg(not(target_os = "macos"))]
        {
            false
        }
    }

    /// Generate embedding for the given text
    ///
    /// Returns a 768-dimensional embedding vector.
    /// Currently returns dummy embeddings; will be replaced with actual MLX implementation.
    pub fn generate_embedding(&self, text: &str) -> Result<Vec<f32>> {
        if self.enabled && self.mlx_available {
            self.generate_mlx_embedding(text)
        } else {
            self.generate_dummy_embedding(text)
        }
    }

    /// Generate embedding using MLX (macOS only)
    ///
    /// TODO: Implement actual MLX-based embedding extraction
    /// For now, returns dummy embeddings even when MLX is available
    #[allow(unused_variables)]
    fn generate_mlx_embedding(&self, text: &str) -> Result<Vec<f32>> {
        #[cfg(all(target_os = "macos", feature = "mlx"))]
        {
            // TODO: Implement MLX-based embedding generation
            // For now, return dummy embeddings
            warn!("MLX embedding generation not yet implemented, using dummy embeddings");
            self.generate_dummy_embedding(text)
        }
        #[cfg(not(all(target_os = "macos", feature = "mlx")))]
        {
            self.generate_dummy_embedding(text)
        }
    }

    /// Generate dummy embedding for development/testing
    ///
    /// Creates a deterministic 768-dimensional embedding based on text content.
    /// Uses a simple hash-based approach to ensure consistency.
    fn generate_dummy_embedding(&self, text: &str) -> Result<Vec<f32>> {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        // Create a deterministic seed from the text
        let mut hasher = DefaultHasher::new();
        text.hash(&mut hasher);
        let seed = hasher.finish();

        // Generate deterministic pseudo-random values
        let mut embedding = Vec::with_capacity(EMBEDDING_DIM);
        let mut rng_state = seed;

        for _ in 0..EMBEDDING_DIM {
            // Simple LCG (Linear Congruential Generator)
            rng_state = rng_state.wrapping_mul(6364136223846793005).wrapping_add(1);
            let value = (rng_state >> 32) as u32;
            // Normalize to [-1, 1] range
            let normalized = (value as f32 / u32::MAX as f32) * 2.0 - 1.0;
            embedding.push(normalized);
        }

        // Normalize the vector to unit length (L2 normalization)
        let magnitude: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
        if magnitude > 0.0 {
            for value in &mut embedding {
                *value /= magnitude;
            }
        }

        Ok(embedding)
    }

    /// Check if embedding service is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Check if MLX is available on this platform
    pub fn is_mlx_available(&self) -> bool {
        self.mlx_available
    }
}

impl Default for EmbeddingService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dummy_embedding_generation() {
        let service = EmbeddingService {
            enabled: false,
            mlx_available: false,
        };

        let embedding = service.generate_embedding("test text").unwrap();
        assert_eq!(embedding.len(), EMBEDDING_DIM);

        // Check that values are normalized (approximately unit length)
        let magnitude: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((magnitude - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_embedding_deterministic() {
        let service = EmbeddingService {
            enabled: false,
            mlx_available: false,
        };

        let embedding1 = service.generate_embedding("test text").unwrap();
        let embedding2 = service.generate_embedding("test text").unwrap();

        // Same text should produce same embedding
        assert_eq!(embedding1, embedding2);
    }

    #[test]
    fn test_embedding_different_text() {
        let service = EmbeddingService {
            enabled: false,
            mlx_available: false,
        };

        let embedding1 = service.generate_embedding("text one").unwrap();
        let embedding2 = service.generate_embedding("text two").unwrap();

        // Different text should produce different embeddings
        assert_ne!(embedding1, embedding2);
    }

    #[test]
    fn test_platform_support_check() {
        let is_supported = EmbeddingService::check_mlx_support();

        #[cfg(all(target_os = "macos", feature = "mlx"))]
        assert!(is_supported);

        #[cfg(not(all(target_os = "macos", feature = "mlx")))]
        assert!(!is_supported);
    }
}
