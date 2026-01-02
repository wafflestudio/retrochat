pub mod database;
pub mod models;
pub mod parsers;
pub mod services;
pub mod tools;
pub mod utils;

pub mod config;
pub mod env;
pub mod error;
pub mod logging;

// Semantic search modules (feature-gated)
#[cfg(feature = "semantic-search")]
pub mod embedding;
#[cfg(feature = "semantic-search")]
pub mod vector_store;

// Re-exports for convenience
pub use database::DatabaseManager;
pub use error::{Result, RetroChatError};
pub use logging::{init_logging, LoggingConfig};

#[cfg(feature = "semantic-search")]
pub use embedding::{EmbeddingConfig, EmbeddingModel, EmbeddingService};
#[cfg(feature = "semantic-search")]
pub use vector_store::VectorStore;
