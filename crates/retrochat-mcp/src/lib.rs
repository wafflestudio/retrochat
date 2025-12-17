//! RetroChat MCP Server
//!
//! A Model Context Protocol server that exposes RetroChat's chat session
//! query and analytics capabilities to AI assistants.

pub mod error;
pub mod server;

// Re-exports for convenience
pub use server::*;
