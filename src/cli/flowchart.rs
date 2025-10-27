use anyhow::{Context, Result};
use clap::Subcommand;
use std::sync::Arc;

use crate::database::DatabaseManager;
use crate::services::{FlowchartService, GoogleAiClient, GoogleAiConfig};

#[derive(Subcommand)]
pub enum FlowchartCommands {
    /// Generate flowchart for a session
    Generate {
        /// Session ID to generate flowchart for
        session_id: String,

        /// Force regenerate even if flowchart exists
        #[arg(short, long)]
        force: bool,
    },
    /// Show flowchart for a session
    Show {
        /// Session ID to show flowchart for
        session_id: String,
    },
    /// Delete flowchart for a session
    Delete {
        /// Session ID to delete flowchart for
        session_id: String,
    },
}

pub async fn handle_flowchart_command(command: FlowchartCommands) -> Result<()> {
    match command {
        FlowchartCommands::Generate { session_id, force } => {
            generate_flowchart(&session_id, force).await
        }
        FlowchartCommands::Show { session_id } => show_flowchart(&session_id).await,
        FlowchartCommands::Delete { session_id } => delete_flowchart(&session_id).await,
    }
}

async fn generate_flowchart(session_id: &str, force: bool) -> Result<()> {
    let db_path = crate::database::config::get_default_db_path()?;
    let db_manager = Arc::new(DatabaseManager::new(&db_path).await?);

    // Get Google AI API key
    let api_key = std::env::var("GOOGLE_AI_API_KEY")
        .context("GOOGLE_AI_API_KEY not set. Please set this environment variable.")?;

    let config = GoogleAiConfig::new(api_key);
    let client = GoogleAiClient::new(config)?;
    let service = FlowchartService::new(db_manager, client);

    println!("Generating flowchart for session: {session_id}");

    if force {
        // Delete existing flowchart first
        service
            .delete_flowchart(session_id)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to delete existing flowchart: {e}"))?;
    }

    let flowchart = service
        .get_or_generate_flowchart(session_id)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to generate flowchart: {e}"))?;

    println!("✓ Flowchart generated successfully!");
    println!("  - {} nodes", flowchart.nodes.len());
    println!("  - {} edges", flowchart.edges.len());
    if let Some(tokens) = flowchart.token_usage {
        println!("  - {tokens} tokens used");
    }
    println!("\nUse 'retrochat flowchart show {session_id}' to view the flowchart");
    Ok(())
}

async fn show_flowchart(session_id: &str) -> Result<()> {
    let db_path = crate::database::config::get_default_db_path()?;
    let db_manager = Arc::new(DatabaseManager::new(&db_path).await?);
    let flowchart_repo = crate::database::FlowchartRepository::new(db_manager);

    let flowcharts = flowchart_repo
        .get_by_session_id(session_id)
        .await
        .map_err(|e| anyhow::anyhow!("Error loading flowchart: {e}"))?;

    if let Some(flowchart) = flowcharts.first() {
        println!("Flowchart for session: {session_id}\n");

        // Render using the same renderer as TUI
        use crate::tui::flowchart_renderer::FlowchartRenderer;
        let renderer = FlowchartRenderer::new(80);
        let lines = renderer.render(flowchart);

        for line in lines {
            // Extract text from Line (ratatui type)
            let text: String = line
                .spans
                .iter()
                .map(|span| span.content.as_ref())
                .collect();
            println!("{text}");
        }

        println!("\nMetadata:");
        println!(
            "  Created: {}",
            flowchart.created_at.format("%Y-%m-%d %H:%M:%S")
        );
        if let Some(tokens) = flowchart.token_usage {
            println!("  Tokens: {tokens}");
        }
        println!("  Nodes: {}", flowchart.nodes.len());
        println!("  Edges: {}", flowchart.edges.len());

        Ok(())
    } else {
        println!("No flowchart found for session: {session_id}");
        println!("Generate one with: retrochat flowchart generate {session_id}");
        Ok(())
    }
}

async fn delete_flowchart(session_id: &str) -> Result<()> {
    let db_path = crate::database::config::get_default_db_path()?;
    let db_manager = Arc::new(DatabaseManager::new(&db_path).await?);
    let flowchart_repo = crate::database::FlowchartRepository::new(db_manager);

    println!("Deleting flowchart for session: {session_id}");

    flowchart_repo
        .delete_by_session_id(session_id)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to delete flowchart: {e}"))?;

    println!("✓ Flowchart deleted successfully");
    Ok(())
}
