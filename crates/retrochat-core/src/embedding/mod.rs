//! Embedding generation module for semantic search.
//!
//! This module provides text embedding generation using FastEmbed-rs
//! for local, CPU-based inference without external API calls.

mod config;
mod models;
mod service;

pub use config::EmbeddingConfig;
pub use models::{EmbeddingModel, ModelInfo};
pub use service::EmbeddingService;
