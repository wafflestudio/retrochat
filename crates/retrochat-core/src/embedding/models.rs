//! Supported embedding models and their metadata.

use std::fmt;

/// Supported embedding models.
///
/// These map to FastEmbed model variants. Quantized versions (with Q suffix)
/// are smaller and faster but may have slightly lower quality.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmbeddingModel {
    /// BGE Small English v1.5 - Good balance of quality and speed.
    /// 384 dimensions, ~33MB model size.
    BGESmallENV15,

    /// BGE Small English v1.5 (Quantized) - Faster, smaller.
    /// 384 dimensions, ~17MB model size.
    BGESmallENV15Q,

    /// All MiniLM L6 v2 - Fast and lightweight.
    /// 384 dimensions, ~23MB model size.
    AllMiniLML6V2,

    /// All MiniLM L6 v2 (Quantized) - Fastest option.
    /// 384 dimensions, ~12MB model size.
    AllMiniLML6V2Q,

    /// BGE Base English v1.5 - Higher quality, slower.
    /// 768 dimensions, ~110MB model size.
    BGEBaseENV15,

    /// BGE Base English v1.5 (Quantized).
    /// 768 dimensions, ~55MB model size.
    BGEBaseENV15Q,
}

impl EmbeddingModel {
    /// Get the number of dimensions for this model's embeddings.
    pub fn dimensions(&self) -> usize {
        match self {
            Self::BGESmallENV15 | Self::BGESmallENV15Q => 384,
            Self::AllMiniLML6V2 | Self::AllMiniLML6V2Q => 384,
            Self::BGEBaseENV15 | Self::BGEBaseENV15Q => 768,
        }
    }

    /// Get the approximate model size in MB.
    pub fn model_size_mb(&self) -> usize {
        match self {
            Self::BGESmallENV15 => 33,
            Self::BGESmallENV15Q => 17,
            Self::AllMiniLML6V2 => 23,
            Self::AllMiniLML6V2Q => 12,
            Self::BGEBaseENV15 => 110,
            Self::BGEBaseENV15Q => 55,
        }
    }

    /// Check if this is a quantized model variant.
    pub fn is_quantized(&self) -> bool {
        matches!(
            self,
            Self::BGESmallENV15Q | Self::AllMiniLML6V2Q | Self::BGEBaseENV15Q
        )
    }

    /// Get the model name as used by FastEmbed.
    pub fn fastembed_name(&self) -> &'static str {
        match self {
            Self::BGESmallENV15 => "BAAI/bge-small-en-v1.5",
            Self::BGESmallENV15Q => "BAAI/bge-small-en-v1.5",
            Self::AllMiniLML6V2 => "sentence-transformers/all-MiniLM-L6-v2",
            Self::AllMiniLML6V2Q => "sentence-transformers/all-MiniLM-L6-v2",
            Self::BGEBaseENV15 => "BAAI/bge-base-en-v1.5",
            Self::BGEBaseENV15Q => "BAAI/bge-base-en-v1.5",
        }
    }
}

impl fmt::Display for EmbeddingModel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BGESmallENV15 => write!(f, "BGESmallENV15"),
            Self::BGESmallENV15Q => write!(f, "BGESmallENV15Q"),
            Self::AllMiniLML6V2 => write!(f, "AllMiniLML6V2"),
            Self::AllMiniLML6V2Q => write!(f, "AllMiniLML6V2Q"),
            Self::BGEBaseENV15 => write!(f, "BGEBaseENV15"),
            Self::BGEBaseENV15Q => write!(f, "BGEBaseENV15Q"),
        }
    }
}

impl std::str::FromStr for EmbeddingModel {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "bgesmallenv15" | "bge-small-en-v1.5" => Ok(Self::BGESmallENV15),
            "bgesmallenv15q" | "bge-small-en-v1.5-q" => Ok(Self::BGESmallENV15Q),
            "allminiml6v2" | "all-minilm-l6-v2" => Ok(Self::AllMiniLML6V2),
            "allminiml6v2q" | "all-minilm-l6-v2-q" => Ok(Self::AllMiniLML6V2Q),
            "bgebaseenv15" | "bge-base-en-v1.5" => Ok(Self::BGEBaseENV15),
            "bgebaseenv15q" | "bge-base-en-v1.5-q" => Ok(Self::BGEBaseENV15Q),
            _ => Err(format!("Unknown embedding model: {s}")),
        }
    }
}

/// Information about the loaded embedding model.
#[derive(Debug, Clone)]
pub struct ModelInfo {
    /// The model variant.
    pub model: EmbeddingModel,

    /// Human-readable model name.
    pub name: String,

    /// Number of embedding dimensions.
    pub dimensions: usize,

    /// Whether the model is quantized.
    pub quantized: bool,
}

impl From<EmbeddingModel> for ModelInfo {
    fn from(model: EmbeddingModel) -> Self {
        Self {
            name: model.to_string(),
            dimensions: model.dimensions(),
            quantized: model.is_quantized(),
            model,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_dimensions() {
        assert_eq!(EmbeddingModel::BGESmallENV15.dimensions(), 384);
        assert_eq!(EmbeddingModel::BGESmallENV15Q.dimensions(), 384);
        assert_eq!(EmbeddingModel::AllMiniLML6V2.dimensions(), 384);
        assert_eq!(EmbeddingModel::BGEBaseENV15.dimensions(), 768);
    }

    #[test]
    fn test_model_is_quantized() {
        assert!(!EmbeddingModel::BGESmallENV15.is_quantized());
        assert!(EmbeddingModel::BGESmallENV15Q.is_quantized());
        assert!(!EmbeddingModel::AllMiniLML6V2.is_quantized());
        assert!(EmbeddingModel::AllMiniLML6V2Q.is_quantized());
    }

    #[test]
    fn test_model_from_str() {
        assert_eq!(
            "BGESmallENV15".parse::<EmbeddingModel>().unwrap(),
            EmbeddingModel::BGESmallENV15
        );
        assert_eq!(
            "bge-small-en-v1.5".parse::<EmbeddingModel>().unwrap(),
            EmbeddingModel::BGESmallENV15
        );
        assert!("invalid".parse::<EmbeddingModel>().is_err());
    }

    #[test]
    fn test_model_display() {
        assert_eq!(EmbeddingModel::BGESmallENV15.to_string(), "BGESmallENV15");
        assert_eq!(EmbeddingModel::AllMiniLML6V2Q.to_string(), "AllMiniLML6V2Q");
    }

    #[test]
    fn test_model_info_from_model() {
        let info = ModelInfo::from(EmbeddingModel::BGESmallENV15Q);
        assert_eq!(info.name, "BGESmallENV15Q");
        assert_eq!(info.dimensions, 384);
        assert!(info.quantized);
    }
}
