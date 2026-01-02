//! LLM client trait definition
//!
//! This module defines the core trait that all LLM providers must implement.

use super::errors::LlmError;
use super::types::{GenerateRequest, GenerateResponse};
use async_trait::async_trait;

/// Provider-agnostic trait for LLM text generation
///
/// This trait abstracts the underlying LLM provider, allowing the analytics
/// system to use Google AI API, Claude Code CLI, or Gemini CLI interchangeably.
#[async_trait]
pub trait LlmClient: Send + Sync {
    /// Generate text completion from a prompt
    ///
    /// # Arguments
    /// * `request` - The generation request containing prompt and optional parameters
    ///
    /// # Returns
    /// * `Ok(GenerateResponse)` - The generated text response with metadata
    /// * `Err(LlmError)` - Provider-specific or transport errors
    async fn generate(&self, request: GenerateRequest) -> Result<GenerateResponse, LlmError>;

    /// Get the provider name for logging and debugging
    fn provider_name(&self) -> &'static str;

    /// Get the model identifier being used
    fn model_name(&self) -> &str;

    /// Check if the client is properly configured and can make requests
    async fn health_check(&self) -> Result<(), LlmError>;

    /// Estimate token count for a given text (rough approximation)
    ///
    /// Default implementation: ~4 characters per token
    fn estimate_tokens(&self, text: &str) -> u32 {
        (text.len() / 4).max(1) as u32
    }
}
