use crate::database::DatabaseManager;
use crate::models::Message;
use crate::services::{QueryService, SearchRequest, SessionDetailRequest, SessionsQueryRequest};
use crate::utils::time_parser;
use anyhow::Result;
use std::sync::Arc;

pub async fn handle_sessions_command(
    page: Option<i32>,
    page_size: Option<i32>,
    provider: Option<String>,
    project: Option<String>,
) -> Result<()> {
    let db_path = crate::database::config::get_default_db_path()?;
    let db_manager = DatabaseManager::new(&db_path).await?;
    let query_service = QueryService::with_database(Arc::new(db_manager));

    let request = SessionsQueryRequest {
        page,
        page_size,
        sort_by: Some("start_time".to_string()),
        sort_order: Some("desc".to_string()),
        filters: Some(crate::services::SessionFilters {
            provider,
            project,
            date_range: None,
            min_messages: None,
            max_messages: None,
        }),
    };

    let response = query_service.query_sessions(request).await?;

    println!(
        "Sessions (Page {}/{}):",
        response.page, response.total_pages
    );
    println!("Total: {} sessions", response.total_count);
    println!();

    for session in response.sessions {
        println!("Session: {}", session.session_id);
        println!("  Provider: {}", session.provider);
        println!(
            "  Project: {}",
            session.project.unwrap_or_else(|| "None".to_string())
        );
        println!("  Messages: {}", session.message_count);
        println!("  Tokens: {}", session.total_tokens.unwrap_or(0));
        println!("  Start: {}", session.start_time);
        println!("  Preview: {}", session.first_message_preview);
        println!();
    }

    Ok(())
}

pub async fn handle_session_detail_command(session_id: String) -> Result<()> {
    let db_path = crate::database::config::get_default_db_path()?;
    let db_manager = DatabaseManager::new(&db_path).await?;
    let query_service = QueryService::with_database(Arc::new(db_manager));

    let request = SessionDetailRequest {
        session_id,
        include_content: Some(true),
        message_limit: Some(50),
        message_offset: Some(0),
    };
    let response = query_service.get_session_detail(request).await?;

    println!("Session Details:");
    println!("  ID: {}", response.session.id);
    println!("  Provider: {}", response.session.provider);
    println!(
        "  Project: {}",
        response
            .session
            .project_name
            .unwrap_or_else(|| "None".to_string())
    );
    println!("  Messages: {}", response.total_message_count);
    println!("  Tokens: {}", response.session.token_count.unwrap_or(0));
    println!("  Start: {}", response.session.start_time);
    println!(
        "  End: {}",
        response
            .session
            .end_time
            .map(|t| t.to_rfc3339())
            .unwrap_or_else(|| "N/A".to_string())
    );
    println!();

    println!("Messages:");
    for (i, message) in response.messages.iter().enumerate() {
        println!("  {}: [{}] {}", i + 1, message.role, message.content);
        if i >= 9 {
            // Show only first 10 messages
            println!("  ... and {} more messages", response.messages.len() - 10);
            break;
        }
    }

    Ok(())
}

pub async fn handle_search_command(query: String, limit: Option<i32>) -> Result<()> {
    let db_path = crate::database::config::get_default_db_path()?;
    let db_manager = DatabaseManager::new(&db_path).await?;
    let query_service = QueryService::with_database(Arc::new(db_manager));

    let request = SearchRequest {
        query,
        page: Some(1),
        page_size: limit,
        date_range: None,
        projects: None,
        providers: None,
        search_type: None,
    };

    let response = query_service.search_messages(request).await?;

    println!(
        "Search Results ({} found in {}ms):",
        response.total_count, response.search_duration_ms
    );
    println!();

    for result in response.results {
        println!(
            "Session: {} | Message: {}",
            result.session_id, result.message_id
        );
        println!(
            "  Provider: {} | Project: {}",
            result.provider,
            result.project.unwrap_or_else(|| "None".to_string())
        );
        println!(
            "  Role: {} | Time: {}",
            result.message_role, result.timestamp
        );
        println!("  Content: {}", result.content_snippet);
        println!();
    }

    Ok(())
}

pub async fn handle_timeline_command(
    since: Option<String>,
    until: Option<String>,
    provider: Option<String>,
    role: Option<String>,
    format: String,
    limit: Option<i32>,
    reverse: bool,
    no_truncate: bool,
    truncate_head: usize,
    truncate_tail: usize,
) -> Result<()> {
    // Parse time specifications
    let from = if let Some(since_str) = since {
        Some(time_parser::parse_time_spec(&since_str)?)
    } else {
        None
    };

    let to = if let Some(until_str) = until {
        Some(time_parser::parse_time_spec(&until_str)?)
    } else {
        None
    };

    // Get database and repository
    let db_path = crate::database::config::get_default_db_path()?;
    let db_manager = DatabaseManager::new(&db_path).await?;
    let message_repo = crate::database::message_repo::MessageRepository::new(&db_manager);

    // Query messages
    let messages = message_repo
        .get_by_time_range(
            from,
            to,
            provider.as_deref(),
            role.as_deref(),
            limit.map(|l| l as i64),
            reverse,
        )
        .await?;

    // Format output
    match format.as_str() {
        "jsonl" => format_jsonl(&messages),
        "compact" | _ => format_compact(&messages, !no_truncate, truncate_head, truncate_tail),
    }

    Ok(())
}

fn format_compact(messages: &[Message], truncate: bool, head_chars: usize, tail_chars: usize) {
    for msg in messages {
        let content = if truncate {
            truncate_message(&msg.content, head_chars, tail_chars)
        } else {
            msg.content.clone()
        };

        let preview = content.replace('\n', " ");
        println!(
            "{} [{:9}] {}",
            msg.timestamp.format("%m-%d %H:%M"),
            msg.role.to_string(),
            preview
        );
    }
}

fn truncate_message(content: &str, head_chars: usize, tail_chars: usize) -> String {
    let chars: Vec<char> = content.chars().collect();
    let total_chars = chars.len();

    // If message is short enough, return as-is
    if total_chars <= head_chars + tail_chars {
        return content.to_string();
    }

    // Extract head and tail
    let head: String = chars.iter().take(head_chars).collect();
    let tail: String = chars.iter().skip(total_chars - tail_chars).collect();

    format!("{} [...] {}", head, tail)
}

fn format_jsonl(messages: &[Message]) {
    for msg in messages {
        if let Ok(json) = serde_json::to_string(msg) {
            println!("{}", json);
        }
    }
}
