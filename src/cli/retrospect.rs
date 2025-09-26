use anyhow::{anyhow, Result};
use chrono::Utc;
use std::collections::HashMap;
use std::fs;
use std::io::{self, Write};
use uuid::Uuid;

use crate::config;
use crate::database::{connection::DatabaseManager, RetrospectionAnalysisRepository};
use crate::models::analysis_request::{AnalysisRequest, RequestStatus};
use crate::services::{PromptService, RetrospectionService};

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
        return Err(anyhow!("Session {} not found", session_id));
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
                "Analysis already exists for session {} with template {}.",
                session_id, template_id
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

    println!(
        "Starting analysis for session {} using template '{}'...",
        session_id, template_id
    );

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
            eprintln!("✗ Analysis failed: {}", e);
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
    let page_size = page_size.unwrap_or(20).max(1).min(100);
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

    println!("Retrospection Analyses (page {}):", page);
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
        .ok_or_else(|| anyhow!("Analysis {} not found", analysis_id))?;

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
        println!("  API Response Metadata: {}", api_metadata);
    }

    println!("\nAnalysis Content:");
    println!("{:-<80}", "");
    println!("{}", analysis.analysis_content);
    println!("{:-<80}", "");

    Ok(())
}

/// Handle template list command
pub async fn handle_template_list_command() -> Result<()> {
    let db_manager = DatabaseManager::new("retrochat.db")?;
    let prompt_service = PromptService::new(db_manager);

    let templates = prompt_service.list_templates(true)?;

    if templates.is_empty() {
        println!("No prompt templates found.");
        println!("Use 'retrospect template create' to add a new template.");
        return Ok(());
    }

    println!("Available Prompt Templates:");
    println!("{:-<100}", "");
    println!("{:<20} {:<30} {:<50}", "ID", "Name", "Description");
    println!("{:-<100}", "");

    for template in templates {
        println!(
            "{:<20} {:<30} {:<50}",
            template.id,
            truncate_string(&template.name, 30),
            truncate_string(&template.description, 50)
        );
    }

    println!("{:-<100}", "");
    println!("Use 'retrospect template show <template_id>' to view template details.");

    Ok(())
}

/// Handle template show command
pub async fn handle_template_show_command(template_id: String) -> Result<()> {
    let db_manager = DatabaseManager::new("retrochat.db")?;
    let prompt_service = PromptService::new(db_manager);

    let template = prompt_service
        .get_template(&template_id)?
        .ok_or_else(|| anyhow!("Template '{}' not found", template_id))?;

    println!("Prompt Template Details");
    println!("{:=<80}", "");
    println!("ID: {}", template.id);
    println!("Name: {}", template.name);
    println!("Description: {}", template.description);
    println!("Category: {}", template.category);
    println!("Default: {}", template.is_default);
    println!(
        "Created: {}",
        template.created_at.format("%Y-%m-%d %H:%M:%S")
    );

    if !template.variables.is_empty() {
        println!("\nVariables:");
        for variable in &template.variables {
            println!(
                "  - {}: {} ({})",
                variable.name,
                variable.description,
                if variable.required {
                    "required"
                } else {
                    "optional"
                }
            );
            if let Some(default) = &variable.default_value {
                println!("    Default: {}", default);
            }
        }
    }

    println!("\nTemplate Content:");
    println!("{:-<80}", "");
    println!("{}", template.template);
    println!("{:-<80}", "");

    Ok(())
}

/// Handle template create command
pub async fn handle_template_create_command(
    id: String,
    name: String,
    description: String,
    content: String,
) -> Result<()> {
    let db_manager = DatabaseManager::new("retrochat.db")?;
    let prompt_service = PromptService::new(db_manager);

    // Handle file content if content starts with @
    let template_content = if content.starts_with('@') {
        let file_path = &content[1..];
        fs::read_to_string(file_path)
            .map_err(|e| anyhow!("Failed to read template file '{}': {}", file_path, e))?
    } else {
        content
    };

    match prompt_service.create_template(
        &id,
        &name,
        &description,
        &template_content,
        Vec::new(), // Variables will be extracted during creation
        "custom",
    ) {
        Ok(_) => {
            println!("✓ Template '{}' created successfully!", id);
        }
        Err(e) => {
            eprintln!("✗ Failed to create template: {}", e);
            return Err(e);
        }
    }

    Ok(())
}

/// Handle template update command
pub async fn handle_template_update_command(
    id: String,
    name: Option<String>,
    description: Option<String>,
    content: Option<String>,
) -> Result<()> {
    let db_manager = DatabaseManager::new("retrochat.db")?;
    let prompt_service = PromptService::new(db_manager);

    // Get existing template
    let _template = prompt_service
        .get_template(&id)?
        .ok_or_else(|| anyhow!("Template '{}' not found", id))?;

    // Prepare update content if provided
    let update_content = if let Some(new_content) = content {
        // Handle file content if content starts with @
        Some(if new_content.starts_with('@') {
            let file_path = &new_content[1..];
            fs::read_to_string(file_path)
                .map_err(|e| anyhow!("Failed to read template file '{}': {}", file_path, e))?
        } else {
            new_content
        })
    } else {
        None
    };

    match prompt_service.update_template(
        &id,
        name,
        description,
        update_content,
        None, // Variables update not implemented in CLI
        None, // Category update not implemented in CLI
    ) {
        Ok(_) => {
            println!("✓ Template '{}' updated successfully!", id);
        }
        Err(e) => {
            eprintln!("✗ Failed to update template: {}", e);
            return Err(e);
        }
    }

    Ok(())
}

/// Handle template delete command
pub async fn handle_template_delete_command(id: String, force: bool) -> Result<()> {
    let db_manager = DatabaseManager::new("retrochat.db")?;
    let prompt_service = PromptService::new(db_manager);

    // Check if template exists
    let _template = prompt_service
        .get_template(&id)?
        .ok_or_else(|| anyhow!("Template '{}' not found", id))?;

    // Confirm deletion unless force is used
    if !force {
        print!("Are you sure you want to delete template '{}'? [y/N]: ", id);
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim().to_lowercase();

        if input != "y" && input != "yes" {
            println!("Template deletion cancelled.");
            return Ok(());
        }
    }

    match prompt_service.delete_template(&id) {
        Ok(_) => {
            println!("✓ Template '{}' deleted successfully!", id);
        }
        Err(e) => {
            eprintln!("✗ Failed to delete template: {}", e);
            return Err(e);
        }
    }

    Ok(())
}

/// Handle template import command
pub async fn handle_template_import_command(file: String, _overwrite: bool) -> Result<()> {
    println!(
        "Template import from '{}' is not yet implemented. Coming in future versions.",
        file
    );
    Ok(())
}

/// Handle template export command
pub async fn handle_template_export_command(
    file: String,
    _templates: Option<Vec<String>>,
) -> Result<()> {
    println!(
        "Template export to '{}' is not yet implemented. Coming in future versions.",
        file
    );
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
                        eprintln!("  Error: {}", error);
                    }
                }
            }
        }
        Err(e) => {
            eprintln!("✗ Failed to process requests: {}", e);
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
            content.push_str(&format!("{}: {}\n\n", role, message_content));
        }

        if content.is_empty() {
            return Err(rusqlite::Error::InvalidColumnType(
                0,
                format!("No messages found for session {}", session_id),
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
