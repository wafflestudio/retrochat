use crate::database::DatabaseManager;
use crate::models::Message;
use crate::services::{QueryService, SearchRequest, SessionDetailRequest, SessionsQueryRequest};
use crate::utils::time_parser;
use anyhow::Result;
use std::sync::Arc;

/// Parameters for timeline command to avoid clippy::too_many_arguments
pub struct TimelineParams {
    pub since: Option<String>,
    pub until: Option<String>,
    pub provider: Option<String>,
    pub role: Option<String>,
    pub format: String,
    pub limit: Option<i32>,
    pub reverse: bool,
    pub no_truncate: bool,
    pub truncate_head: usize,
    pub truncate_tail: usize,
    pub no_tool: bool,
}

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

pub async fn handle_search_command(
    query: String,
    limit: Option<i32>,
    since: Option<String>,
    until: Option<String>,
) -> Result<()> {
    let db_path = crate::database::config::get_default_db_path()?;
    let db_manager = DatabaseManager::new(&db_path).await?;
    let query_service = QueryService::with_database(Arc::new(db_manager));

    // Parse time specifications if provided
    let date_range = if since.is_some() || until.is_some() {
        let start_date = if let Some(since_str) = since {
            time_parser::parse_time_spec(&since_str)?.to_rfc3339()
        } else {
            // Use a very old date as default start
            chrono::DateTime::parse_from_rfc3339("1970-01-01T00:00:00Z")?
                .with_timezone(&chrono::Utc)
                .to_rfc3339()
        };

        let end_date = if let Some(until_str) = until {
            time_parser::parse_time_spec(&until_str)?.to_rfc3339()
        } else {
            // Use now as default end
            chrono::Utc::now().to_rfc3339()
        };

        Some(crate::services::DateRange {
            start_date,
            end_date,
        })
    } else {
        None
    };

    let request = SearchRequest {
        query,
        page: Some(1),
        page_size: limit,
        date_range,
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

pub async fn handle_timeline_command(params: TimelineParams) -> Result<()> {
    // Parse time specifications
    let from = if let Some(since_str) = params.since {
        Some(time_parser::parse_time_spec(&since_str)?)
    } else {
        None
    };

    let to = if let Some(until_str) = params.until {
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
            params.provider.as_deref(),
            params.role.as_deref(),
            params.limit.map(|l| l as i64),
            params.reverse,
        )
        .await?;

    // Format output
    match params.format.as_str() {
        "jsonl" => format_jsonl(&messages, params.no_tool),
        _ => format_compact(
            &messages,
            !params.no_truncate,
            params.truncate_head,
            params.truncate_tail,
            params.no_tool,
        ),
    }

    Ok(())
}

fn format_compact(
    messages: &[Message],
    truncate: bool,
    head_chars: usize,
    tail_chars: usize,
    no_tool: bool,
) {
    for msg in messages {
        // Filter out tool messages if no_tool is enabled
        if no_tool && is_tool_message(&msg.content) {
            continue;
        }

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

    format!("{head} [...] {tail}")
}

fn format_jsonl(messages: &[Message], no_tool: bool) {
    for msg in messages {
        // Filter out tool messages if no_tool is enabled
        if no_tool && is_tool_message(&msg.content) {
            continue;
        }

        if let Ok(json) = serde_json::to_string(msg) {
            println!("{json}");
        }
    }
}

/// Check if a message is a tool use or tool result message
fn is_tool_message(content: &str) -> bool {
    content.starts_with("[Tool Use:")
        || content.starts_with("[Tool Result]")
        || content.trim().starts_with("[Tool Use:")
        || content.trim().starts_with("[Tool Result]")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_tool_message_tool_use() {
        assert!(is_tool_message("[Tool Use: Read]"));
        assert!(is_tool_message("[Tool Use: Grep]"));
        assert!(is_tool_message("  [Tool Use: Edit]")); // with leading whitespace
    }

    #[test]
    fn test_is_tool_message_tool_result() {
        assert!(is_tool_message("[Tool Result]"));
        assert!(is_tool_message("  [Tool Result]")); // with leading whitespace
    }

    #[test]
    fn test_is_tool_message_regular_message() {
        assert!(!is_tool_message("This is a regular message"));
        assert!(!is_tool_message("User message content"));
        assert!(!is_tool_message("Assistant response"));
        assert!(!is_tool_message(""));
    }

    #[test]
    fn test_is_tool_message_false_positives() {
        // Should not match if [Tool is in the middle
        assert!(!is_tool_message("Here is [Tool Use: something]"));
        assert!(!is_tool_message("Text before [Tool Result]"));
    }
}
