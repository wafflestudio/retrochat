use crate::dto::{
    FileMetadataItem, MessageItem, SearchResultItem, SessionDetail, SessionListItem,
    ToolOperationItem,
};
use crate::AppState;
use retrochat::database::ToolOperationRepository;
use retrochat::services::{
    SearchRequest, SessionDetailRequest, SessionFilters, SessionsQueryRequest,
};
use std::sync::Arc;
use tauri::State;
use tokio::sync::Mutex;

#[tauri::command]
pub async fn get_sessions(
    state: State<'_, Arc<Mutex<AppState>>>,
    page: Option<i32>,
    page_size: Option<i32>,
    provider: Option<String>,
) -> Result<Vec<SessionListItem>, String> {
    log::info!(
        "get_sessions called - page: {:?}, page_size: {:?}, provider: {:?}",
        page,
        page_size,
        provider
    );

    let state = state.lock().await;

    let filters = provider.as_ref().map(|p| {
        log::debug!("Applying provider filter: {}", p);
        SessionFilters {
            provider: Some(p.clone()),
            project: None,
            date_range: None,
            min_messages: None,
            max_messages: None,
        }
    });

    let request = SessionsQueryRequest {
        page,
        page_size,
        sort_by: Some("start_time".to_string()),
        sort_order: Some("desc".to_string()),
        filters,
    };

    let response = state
        .query_service
        .query_sessions(request)
        .await
        .map_err(|e| {
            log::error!("Failed to query sessions: {}", e);
            e.to_string()
        })?;

    let session_count = response.sessions.len();
    log::info!("Successfully retrieved {} sessions", session_count);

    Ok(response
        .sessions
        .into_iter()
        .map(|s| SessionListItem {
            id: s.session_id,
            provider: s.provider,
            project_name: s.project,
            created_at: s.start_time,
            updated_at: s.end_time,
            message_count: s.message_count,
        })
        .collect())
}

#[tauri::command]
pub async fn get_session_detail(
    state: State<'_, Arc<Mutex<AppState>>>,
    session_id: String,
) -> Result<SessionDetail, String> {
    log::info!("get_session_detail called - session_id: {}", session_id);

    let state_guard = state.lock().await;

    let request = SessionDetailRequest {
        session_id: session_id.clone(),
        include_content: Some(true),
        message_limit: None,
        message_offset: None,
    };

    log::debug!("Fetching session detail from query service");
    let response = state_guard
        .query_service
        .get_session_detail(request)
        .await
        .map_err(|e| {
            log::error!("Failed to get session detail: {}", e);
            e.to_string()
        })?;

    // Get tool operations for all messages in this session
    log::debug!("Parsing session UUID");
    let session_uuid = uuid::Uuid::parse_str(&session_id).map_err(|e| {
        log::error!("Failed to parse session UUID: {}", e);
        e.to_string()
    })?;

    log::debug!("Fetching tool operations for session");
    let tool_op_repo = ToolOperationRepository::new(&state_guard.db_manager);
    let tool_operations = tool_op_repo
        .get_by_session(&session_uuid)
        .await
        .map_err(|e| {
            log::error!("Failed to fetch tool operations: {}", e);
            e.to_string()
        })?;

    log::debug!(
        "Found {} tool operations for session",
        tool_operations.len()
    );

    // Create a map of message_id -> tool_operation
    log::debug!("Building tool operation map");
    let mut tool_op_map = std::collections::HashMap::new();
    for tool_op in tool_operations {
        // Find the message that references this tool operation
        for msg in &response.messages {
            if msg.tool_operation_id == Some(tool_op.id) {
                tool_op_map.insert(msg.id, tool_op.clone());
                break;
            }
        }
    }

    log::info!(
        "Successfully retrieved session detail with {} messages",
        response.messages.len()
    );

    Ok(SessionDetail {
        id: response.session.id.to_string(),
        provider: response.session.provider.to_string(),
        project_name: response.session.project_name,
        created_at: response.session.start_time.to_rfc3339(),
        updated_at: response
            .session
            .end_time
            .map(|dt| dt.to_rfc3339())
            .unwrap_or_else(|| response.session.updated_at.to_rfc3339()),
        messages: response
            .messages
            .into_iter()
            .map(|m| {
                let tool_operation = tool_op_map.get(&m.id).map(|op| ToolOperationItem {
                    id: op.id.to_string(),
                    tool_use_id: op.tool_use_id.clone(),
                    tool_name: op.tool_name.clone(),
                    timestamp: op.timestamp.to_rfc3339(),
                    success: op.success,
                    result_summary: op.result_summary.clone(),
                    file_metadata: op.file_metadata.as_ref().map(|fm| FileMetadataItem {
                        file_path: fm.file_path.clone(),
                        file_extension: fm.file_extension.clone(),
                        is_code_file: fm.is_code_file,
                        lines_added: fm.lines_added,
                        lines_removed: fm.lines_removed,
                    }),
                    bash_metadata: op
                        .bash_metadata
                        .clone()
                        .map(|bm| serde_json::to_value(bm).unwrap_or(serde_json::Value::Null)),
                });

                MessageItem {
                    id: m.id.to_string(),
                    role: m.role.to_string(),
                    content: m.content,
                    timestamp: m.timestamp.to_rfc3339(),
                    message_type: m.message_type.to_string(),
                    tool_operation,
                }
            })
            .collect(),
    })
}

#[tauri::command]
pub async fn search_messages(
    state: State<'_, Arc<Mutex<AppState>>>,
    query: String,
    limit: Option<i32>,
) -> Result<Vec<SearchResultItem>, String> {
    log::info!(
        "search_messages called - query: '{}', limit: {:?}",
        query,
        limit
    );

    let state = state.lock().await;

    let request = SearchRequest {
        query: query.clone(),
        providers: None,
        projects: None,
        date_range: None,
        search_type: None,
        page: Some(1),
        page_size: limit,
    };

    log::debug!("Executing message search");
    let response = state
        .query_service
        .search_messages(request)
        .await
        .map_err(|e| {
            log::error!("Failed to search messages: {}", e);
            e.to_string()
        })?;

    let result_count = response.results.len();
    log::info!("Search completed - found {} results", result_count);

    Ok(response
        .results
        .into_iter()
        .map(|r| SearchResultItem {
            session_id: r.session_id,
            message_id: r.message_id,
            content: r.content_snippet,
            role: "User".to_string(), // Default role since SearchResult doesn't include it
            timestamp: r.timestamp,
            provider: r.provider,
        })
        .collect())
}

#[tauri::command]
pub async fn get_providers(_state: State<'_, Arc<Mutex<AppState>>>) -> Result<Vec<String>, String> {
    log::debug!("get_providers called");
    // Return available providers
    Ok(vec![
        "Claude Code".to_string(),
        "Gemini CLI".to_string(),
        "Codex".to_string(),
    ])
}
