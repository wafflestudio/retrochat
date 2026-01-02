//! Multi-LLM Provider Support
//!
//! This module provides a provider-agnostic abstraction for LLM operations,
//! enabling RetroChat's analysis feature to use different LLM backends:
//!
//! - **Google AI API**: Remote API calls (existing)
//! - **Claude Code CLI**: Local subprocess via `claude -p`
//! - **Gemini CLI**: Local subprocess via `gemini`
//!
//! # Usage
//!
//! ```no_run
//! use retrochat_core::services::llm::{LlmClientFactory, LlmConfig, LlmProvider, GenerateRequest};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create client from environment
//! let client = LlmClientFactory::from_env()?;
//!
//! // Or create with explicit configuration
//! let config = LlmConfig::claude_code();
//! let client = LlmClientFactory::create(config)?;
//!
//! // Generate text
//! let request = GenerateRequest::new("Analyze this code...".to_string())
//!     .with_max_tokens(1024);
//! let response = client.generate(request).await?;
//! println!("{}", response.text);
//! # Ok(())
//! # }
//! ```

pub mod adapters;
mod errors;
mod factory;
pub mod subprocess;
mod traits;
mod types;

// Re-export main types
pub use errors::LlmError;
pub use factory::LlmClientFactory;
pub use traits::LlmClient;
pub use types::{GenerateRequest, GenerateResponse, LlmConfig, LlmProvider, TokenUsage};
