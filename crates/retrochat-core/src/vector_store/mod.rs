//! Vector store module for semantic search using LanceDB.
//!
//! This module provides vector storage and similarity search capabilities
//! using LanceDB as the embedded vector database.

mod models;
mod schemas;
mod store;

pub use models::{SessionEmbedding, TurnEmbedding};
pub use store::VectorStore;
