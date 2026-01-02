//! LLM provider adapters
//!
//! This module provides adapters for different LLM providers:
//! - `GoogleAiAdapter`: Wraps the existing Google AI client
//! - `ClaudeCodeClient`: Invokes Claude Code CLI as subprocess
//! - `GeminiCliClient`: Invokes Gemini CLI as subprocess

mod claude_code;
mod gemini_cli;
mod google_ai;

pub use claude_code::ClaudeCodeClient;
pub use gemini_cli::GeminiCliClient;
pub use google_ai::GoogleAiAdapter;
