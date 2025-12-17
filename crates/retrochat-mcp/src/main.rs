//! RetroChat MCP Server
//!
//! A Model Context Protocol server that exposes RetroChat's chat session
//! query and analytics capabilities to AI assistants.

use retrochat_mcp::RetroChatMcpServer;
use rmcp::transport::stdio;
use rmcp::ServiceExt;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging to stderr (won't interfere with stdio transport)
    // Use RUST_LOG environment variable to control log level
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(std::io::stderr) // Log to stderr, not stdout
                .with_target(false)
                .with_thread_ids(false)
                .with_file(false)
                .with_line_number(false),
        )
        .init();

    tracing::info!(
        "Starting RetroChat MCP Server v{}",
        env!("CARGO_PKG_VERSION")
    );

    // Create the server
    let server = RetroChatMcpServer::new().await.map_err(|e| {
        tracing::error!("Failed to initialize server: {}", e);
        e
    })?;

    tracing::info!("Server initialized successfully");

    // Start serving with stdio transport
    let service = server.serve(stdio()).await.map_err(|e| {
        tracing::error!("Failed to start server: {}", e);
        e
    })?;

    tracing::info!("MCP server running on stdio transport");

    // Wait for the service to complete
    service.waiting().await.map_err(|e| {
        tracing::error!("Server error: {}", e);
        e
    })?;

    tracing::info!("Server shutting down gracefully");

    Ok(())
}
