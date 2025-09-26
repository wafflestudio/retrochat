use anyhow::{anyhow, Result};
use chrono::Utc;
use std::collections::HashMap;
use uuid::Uuid;

use crate::config;
use crate::database::{connection::DatabaseManager, RetrospectionAnalysisRepository};
use crate::models::analysis_request::{AnalysisRequest, RequestStatus};
use crate::services::RetrospectionService;

/// Handle retrospect analyze command
pub async fn handle_analyze_command(
    session_id: String,
    template: Option<String>,
    force: bool,
) -> Result<()> {
    config::validate_environment()?;
    let db_manager = DatabaseManager::new("retrochat.db")?;

    // Parse session ID
    let session_uuid =
        Uuid::parse_str(&session_id).map_err(|_| anyhow!("Invalid session ID format"))?;

    // Check if session exists
    let session_exists = db_manager.with_connection(|conn| {
        let mut stmt = conn.prepare("SELECT id FROM chat_sessions WHERE id = ?1")?;
        stmt.exists([session_uuid.to_string()])
    })?;
    if !session_exists {
        return Err(anyhow!("Session {session_id} not found"));
    }

    // Use default template if not specified
    let template_id = template.unwrap_or_else(|| "session_summary".to_string());

    // Check if analysis already exists and force is not set
    if !force {
        let analysis_exists = db_manager.with_connection(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id FROM retrospection_analyses WHERE session_id = ?1 AND prompt_template_id = ?2"
            )?;
            stmt.exists([session_uuid.to_string(), template_id.clone()])
        })?;
        if analysis_exists {
            println!(
                "Analysis already exists for session {session_id} with template {template_id}."
            );
            println!(
                "Use --force to re-analyze or use 'retrospect show' to view existing analysis."
            );
            return Ok(());
        }
    }

    // Create retrospection service
    let retrospection_service = RetrospectionService::new(db_manager.clone())?;

    // Get chat content for the session
    let chat_content = get_session_content(&db_manager, session_uuid).await?;

    // Create analysis request
    let mut variables = HashMap::new();
    variables.insert("chat_content".to_string(), chat_content);

    let request = AnalysisRequest {
        id: Uuid::new_v4(),
        session_id: session_uuid,
        prompt_template_id: template_id.clone(),
        template_variables: variables,
        status: RequestStatus::Queued,
        error_message: None,
        created_at: Utc::now(),
        started_at: None,
        completed_at: None,
    };

    println!("Starting analysis for session {session_id} using template '{template_id}'...");

    // Process the analysis request
    match retrospection_service
        .process_analysis_request(request)
        .await
    {
        Ok(analysis) => {
            println!("✓ Analysis completed successfully!");
            println!("Analysis ID: {}", analysis.id);
            println!("Template: {}", analysis.prompt_template_id);
            println!("Status: {:?}", analysis.status);
            println!(
                "Content length: {} characters",
                analysis.analysis_content.len()
            );

            if analysis.metadata.api_response_metadata.is_some() {
                println!(
                    "Tokens used: {} prompt + {} completion = {} total",
                    analysis.metadata.prompt_tokens,
                    analysis.metadata.completion_tokens,
                    analysis.metadata.total_tokens
                );
                println!("Estimated cost: ${:.6}", analysis.metadata.estimated_cost);
                println!("Execution time: {}ms", analysis.metadata.execution_time_ms);
            }

            println!(
                "\nUse 'retrospect show {}' to view the full analysis.",
                analysis.id
            );
        }
        Err(e) => {
            eprintln!("✗ Analysis failed: {e}");
            return Err(e);
        }
    }

    Ok(())
}

/// Handle retrospect list command
pub async fn handle_list_command(
    session: Option<String>,
    template: Option<String>,
    page: Option<i32>,
    page_size: Option<i32>,
) -> Result<()> {
    let db_manager = DatabaseManager::new("retrochat.db")?;
    let retrospection_service = RetrospectionService::new(db_manager.clone())?;

    let page = page.unwrap_or(1).max(1);
    let page_size = page_size.unwrap_or(20).clamp(1, 100);
    let offset = (page - 1) * page_size;

    // Parse session ID if provided
    let session_filter = if let Some(session_id) = session {
        Some(Uuid::parse_str(&session_id).map_err(|_| anyhow!("Invalid session ID format"))?)
    } else {
        None
    };

    // Get analyses based on filters
    let analyses = if let Some(session_id) = session_filter {
        // Filter by session ID
        retrospection_service
            .get_analyses_for_session(session_id)
            .await?
    } else if let Some(template_id) = template {
        // We need to use direct database access for template filtering
        {
            db_manager.with_connection_anyhow(|conn| {
                let repo = RetrospectionAnalysisRepository::new(conn);
                repo.find_by_template_id(&template_id)
            })?
        }
    } else {
        // List all with pagination
        {
            db_manager.with_connection_anyhow(|conn| {
                let repo = RetrospectionAnalysisRepository::new(conn);
                repo.list_all(Some(page_size as u32), Some(offset as u32))
            })?
        }
    };

    if analyses.is_empty() {
        println!("No retrospection analyses found.");
        return Ok(());
    }

    println!("Retrospection Analyses (page {page}):");
    println!("{:-<120}", "");
    println!(
        "{:<36} {:<36} {:<20} {:<12} {:<20}",
        "Analysis ID", "Session ID", "Template", "Status", "Created"
    );
    println!("{:-<120}", "");

    for analysis in analyses {
        let created = analysis.created_at.format("%Y-%m-%d %H:%M:%S").to_string();
        println!(
            "{:<36} {:<36} {:<20} {:<12} {:<20}",
            analysis.id,
            analysis.session_id,
            truncate_string(&analysis.prompt_template_id, 20),
            format!("{:?}", analysis.status),
            created
        );
    }

    println!("{:-<120}", "");
    println!("Use 'retrospect show <analysis_id>' to view full details.");

    Ok(())
}

/// Handle retrospect show command
pub async fn handle_show_command(analysis_id: String) -> Result<()> {
    let db_manager = DatabaseManager::new("retrochat.db")?;
    let retrospection_service = RetrospectionService::new(db_manager.clone())?;

    // Parse analysis ID
    let analysis_uuid =
        Uuid::parse_str(&analysis_id).map_err(|_| anyhow!("Invalid analysis ID format"))?;

    // Get analysis by ID
    let analysis = retrospection_service
        .get_analysis(analysis_uuid)
        .await?
        .ok_or_else(|| anyhow!("Analysis {analysis_id} not found"))?;

    // Display analysis details
    println!("Retrospection Analysis Details");
    println!("{:=<80}", "");
    println!("Analysis ID: {}", analysis.id);
    println!("Session ID: {}", analysis.session_id);
    println!("Template: {}", analysis.prompt_template_id);
    println!("Status: {:?}", analysis.status);
    println!(
        "Created: {}",
        analysis.created_at.format("%Y-%m-%d %H:%M:%S")
    );
    println!(
        "Updated: {}",
        analysis.updated_at.format("%Y-%m-%d %H:%M:%S")
    );

    println!("\nMetadata:");
    println!("  LLM Service: {}", analysis.metadata.llm_service);
    println!("  Prompt Tokens: {}", analysis.metadata.prompt_tokens);
    println!(
        "  Completion Tokens: {}",
        analysis.metadata.completion_tokens
    );
    println!("  Total Tokens: {}", analysis.metadata.total_tokens);
    println!("  Estimated Cost: ${:.6}", analysis.metadata.estimated_cost);
    println!(
        "  Execution Time: {}ms",
        analysis.metadata.execution_time_ms
    );

    if let Some(api_metadata) = &analysis.metadata.api_response_metadata {
        println!("  API Response Metadata: {api_metadata}");
    }

    println!("\nAnalysis Content:");
    println!("{:-<80}", "");
    println!("{}", analysis.analysis_content);
    println!("{:-<80}", "");

    Ok(())
}

/// Handle template list command
pub async fn handle_template_list_command() -> Result<()> {
    println!("Template management has been simplified in this version.");
    println!("The application now uses a single hardcoded prompt for all analyses.");
    Ok(())
}

/// Handle template show command
pub async fn handle_template_show_command(_template_id: String) -> Result<()> {
    println!("Template management has been simplified.");
    println!(
        "This version uses a hardcoded prompt. Use 'retrospect analyze <session-id>' instead."
    );
    Ok(())
}

/// Handle template create command
pub async fn handle_template_create_command(
    _id: String,
    _name: String,
    _description: String,
    _content: String,
) -> Result<()> {
    println!("Template creation is not available in the simplified version.");
    println!("The application uses a hardcoded prompt for all analyses.");
    Ok(())
}

/// Handle template update command
pub async fn handle_template_update_command(
    _id: String,
    _name: Option<String>,
    _description: Option<String>,
    _content: Option<String>,
) -> Result<()> {
    println!("Template management is not available in the simplified version.");
    Ok(())
}

/// Handle template delete command
pub async fn handle_template_delete_command(_id: String, _force: bool) -> Result<()> {
    println!("Template management is not available in the simplified version.");
    Ok(())
}

/// Handle template import command
pub async fn handle_template_import_command(_file: String, _overwrite: bool) -> Result<()> {
    println!("Template management is not available in the simplified version.");
    Ok(())
}

/// Handle template export command
pub async fn handle_template_export_command(
    _file: String,
    _templates: Option<Vec<String>>,
) -> Result<()> {
    println!("Template management is not available in the simplified version.");
    Ok(())
}

/// Handle process command
pub async fn handle_process_command(_limit: Option<i32>, _force: bool) -> Result<()> {
    config::validate_environment()?;
    let db_manager = DatabaseManager::new("retrochat.db")?;

    let retrospection_service = RetrospectionService::new(db_manager.clone())?;

    let _limit = _limit.map(|l| l.max(1));

    println!("Processing pending analysis requests...");

    match retrospection_service.process_pending_requests().await {
        Ok(result) => {
            if result.processed == 0 {
                println!("No pending requests to process.");
            } else {
                println!("✓ {}", result.get_summary());
                if result.has_failures() {
                    for error in &result.errors {
                        eprintln!("  Error: {error}");
                    }
                }
            }
        }
        Err(e) => {
            eprintln!("✗ Failed to process requests: {e}");
            return Err(e);
        }
    }

    Ok(())
}

/// Helper function to get session content for analysis
async fn get_session_content(db_manager: &DatabaseManager, session_id: Uuid) -> Result<String> {
    db_manager.with_connection(|conn| {
        let mut stmt = conn.prepare(
            "SELECT role, content FROM messages WHERE session_id = ?1 ORDER BY created_at ASC",
        )?;

        let message_rows = stmt.query_map([session_id.to_string()], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;

        let mut content = String::new();
        for message_result in message_rows {
            let (role, message_content) = message_result?;
            content.push_str(&format!("{role}: {message_content}\n\n"));
        }

        if content.is_empty() {
            return Err(rusqlite::Error::InvalidColumnType(
                0,
                format!("No messages found for session {session_id}"),
                rusqlite::types::Type::Text,
            ));
        }

        Ok(content)
    })
}

/// Helper function to truncate strings for display
fn truncate_string(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[0..max_len.saturating_sub(3)])
    }
}
