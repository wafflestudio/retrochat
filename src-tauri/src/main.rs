// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use retrochat::database::{config, DatabaseManager, MessageRepository, ToolOperationRepository};
use retrochat::services::{
    google_ai::{GoogleAiClient, GoogleAiConfig},
    AnalyticsRequestService, QueryService, SearchRequest, SessionDetailRequest, SessionFilters,
    SessionsQueryRequest,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::State;
use tokio::sync::Mutex;

// Application state
pub struct AppState {
    db_manager: Arc<DatabaseManager>,
    query_service: Arc<QueryService>,
    analytics_service: Option<Arc<AnalyticsRequestService>>,
}

// DTOs for frontend communication
#[derive(Debug, Serialize, Deserialize)]
pub struct SessionListItem {
    pub id: String,
    pub provider: String,
    pub project_name: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub message_count: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SessionDetail {
    pub id: String,
    pub provider: String,
    pub project_name: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub messages: Vec<MessageItem>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MessageItem {
    pub id: String,
    pub role: String,
    pub content: String,
    pub timestamp: String,
    pub message_type: String,
    pub tool_operation: Option<ToolOperationItem>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ToolOperationItem {
    pub id: String,
    pub tool_use_id: String,
    pub tool_name: String,
    pub timestamp: String,
    pub success: Option<bool>,
    pub result_summary: Option<String>,
    pub file_metadata: Option<FileMetadataItem>,
    pub bash_metadata: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FileMetadataItem {
    pub file_path: String,
    pub file_extension: Option<String>,
    pub is_code_file: Option<bool>,
    pub lines_added: Option<i32>,
    pub lines_removed: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchResultItem {
    pub session_id: String,
    pub message_id: String,
    pub content: String,
    pub role: String,
    pub timestamp: String,
    pub provider: String,
}

// Analysis DTOs
#[derive(Debug, Serialize, Deserialize)]
pub struct AnalyticsRequestItem {
    pub id: String,
    pub session_id: String,
    pub status: String,
    pub started_at: String,
    pub completed_at: Option<String>,
    pub error_message: Option<String>,
}

// Tauri Commands
#[tauri::command]
async fn get_sessions(
    state: State<'_, Arc<Mutex<AppState>>>,
    page: Option<i32>,
    page_size: Option<i32>,
    provider: Option<String>,
) -> Result<Vec<SessionListItem>, String> {
    let state = state.lock().await;

    let filters = provider.map(|p| SessionFilters {
        provider: Some(p),
        project: None,
        date_range: None,
        min_messages: None,
        max_messages: None,
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
        .map_err(|e| e.to_string())?;

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
async fn get_session_detail(
    state: State<'_, Arc<Mutex<AppState>>>,
    session_id: String,
) -> Result<SessionDetail, String> {
    let state_guard = state.lock().await;

    let request = SessionDetailRequest {
        session_id: session_id.clone(),
        include_content: Some(true),
        message_limit: None,
        message_offset: None,
    };

    let response = state_guard
        .query_service
        .get_session_detail(request)
        .await
        .map_err(|e| e.to_string())?;

    // Get tool operations for all messages in this session
    let session_uuid = uuid::Uuid::parse_str(&session_id).map_err(|e| e.to_string())?;
    let tool_op_repo = ToolOperationRepository::new(&state_guard.db_manager);
    let tool_operations = tool_op_repo
        .get_by_session(&session_uuid)
        .await
        .map_err(|e| e.to_string())?;

    // Create a map of message_id -> tool_operation
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
async fn search_messages(
    state: State<'_, Arc<Mutex<AppState>>>,
    query: String,
    limit: Option<i32>,
) -> Result<Vec<SearchResultItem>, String> {
    let state = state.lock().await;

    let request = SearchRequest {
        query,
        providers: None,
        projects: None,
        date_range: None,
        search_type: None,
        page: Some(1),
        page_size: limit,
    };

    let response = state
        .query_service
        .search_messages(request)
        .await
        .map_err(|e| e.to_string())?;

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
async fn get_providers(_state: State<'_, Arc<Mutex<AppState>>>) -> Result<Vec<String>, String> {
    // Return available providers
    Ok(vec![
        "Claude".to_string(),
        "Gemini".to_string(),
        "Codex".to_string(),
    ])
}

// Analysis Commands

/// Convenience command: Create and execute analysis in one call
#[tauri::command]
async fn analyze_session(
    state: State<'_, Arc<Mutex<AppState>>>,
    session_id: String,
    custom_prompt: Option<String>,
) -> Result<AnalyticsRequestItem, String> {
    let state_guard = state.lock().await;

    let analytics_service = state_guard.analytics_service.as_ref().ok_or(
        "Analytics service not available. Please set GOOGLE_AI_API_KEY environment variable.",
    )?;

    // Create the request
    let request = analytics_service
        .create_analysis_request(session_id, None, custom_prompt)
        .await
        .map_err(|e| e.to_string())?;

    let request_id = request.id.clone();

    // Execute immediately
    analytics_service
        .execute_analysis(request_id.clone())
        .await
        .map_err(|e| e.to_string())?;

    // Get the updated status
    let completed_request = analytics_service
        .get_analysis_status(request_id)
        .await
        .map_err(|e| e.to_string())?;

    Ok(AnalyticsRequestItem {
        id: completed_request.id,
        session_id: completed_request.session_id,
        status: completed_request.status.to_string(),
        started_at: completed_request.started_at.to_rfc3339(),
        completed_at: completed_request.completed_at.map(|dt| dt.to_rfc3339()),
        error_message: completed_request.error_message,
    })
}

/// Create an analysis request without executing it (for advanced use cases)
#[tauri::command]
async fn create_analysis(
    state: State<'_, Arc<Mutex<AppState>>>,
    session_id: String,
    custom_prompt: Option<String>,
) -> Result<AnalyticsRequestItem, String> {
    let state_guard = state.lock().await;

    let analytics_service = state_guard.analytics_service.as_ref().ok_or(
        "Analytics service not available. Please set GOOGLE_AI_API_KEY environment variable.",
    )?;

    let request = analytics_service
        .create_analysis_request(session_id, None, custom_prompt)
        .await
        .map_err(|e| e.to_string())?;

    Ok(AnalyticsRequestItem {
        id: request.id,
        session_id: request.session_id,
        status: request.status.to_string(),
        started_at: request.started_at.to_rfc3339(),
        completed_at: request.completed_at.map(|dt| dt.to_rfc3339()),
        error_message: request.error_message,
    })
}

#[tauri::command]
async fn run_analysis(
    state: State<'_, Arc<Mutex<AppState>>>,
    request_id: String,
) -> Result<String, String> {
    let state_guard = state.lock().await;

    let analytics_service = state_guard
        .analytics_service
        .as_ref()
        .ok_or("Analytics service not available")?;

    analytics_service
        .execute_analysis(request_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_analysis_status(
    state: State<'_, Arc<Mutex<AppState>>>,
    request_id: String,
) -> Result<AnalyticsRequestItem, String> {
    let state_guard = state.lock().await;

    let analytics_service = state_guard
        .analytics_service
        .as_ref()
        .ok_or("Analytics service not available")?;

    let request = analytics_service
        .get_analysis_status(request_id)
        .await
        .map_err(|e| e.to_string())?;

    Ok(AnalyticsRequestItem {
        id: request.id,
        session_id: request.session_id,
        status: request.status.to_string(),
        started_at: request.started_at.to_rfc3339(),
        completed_at: request.completed_at.map(|dt| dt.to_rfc3339()),
        error_message: request.error_message,
    })
}

#[tauri::command]
async fn get_analysis_result(
    state: State<'_, Arc<Mutex<AppState>>>,
    request_id: String,
) -> Result<Option<serde_json::Value>, String> {
    let state_guard = state.lock().await;

    let analytics_service = state_guard
        .analytics_service
        .as_ref()
        .ok_or("Analytics service not available")?;

    let result = analytics_service
        .get_analysis_result(request_id)
        .await
        .map_err(|e| e.to_string())?;

    // Convert Analytics to JSON
    Ok(result.map(|analytics| serde_json::to_value(analytics).unwrap_or(serde_json::Value::Null)))
}

#[tauri::command]
async fn list_analyses(
    state: State<'_, Arc<Mutex<AppState>>>,
    session_id: Option<String>,
    limit: Option<usize>,
) -> Result<Vec<AnalyticsRequestItem>, String> {
    let state_guard = state.lock().await;

    let analytics_service = state_guard
        .analytics_service
        .as_ref()
        .ok_or("Analytics service not available")?;

    let requests = analytics_service
        .list_analyses(session_id, limit)
        .await
        .map_err(|e| e.to_string())?;

    Ok(requests
        .into_iter()
        .map(|r| AnalyticsRequestItem {
            id: r.id,
            session_id: r.session_id,
            status: r.status.to_string(),
            started_at: r.started_at.to_rfc3339(),
            completed_at: r.completed_at.map(|dt| dt.to_rfc3339()),
            error_message: r.error_message,
        })
        .collect())
}

#[tauri::command]
async fn cancel_analysis(
    state: State<'_, Arc<Mutex<AppState>>>,
    request_id: String,
) -> Result<(), String> {
    let state_guard = state.lock().await;

    let analytics_service = state_guard
        .analytics_service
        .as_ref()
        .ok_or("Analytics service not available")?;

    analytics_service
        .cancel_analysis(request_id)
        .await
        .map_err(|e| e.to_string())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize database
    let db_path = config::get_default_db_path()?;
    let db_manager = Arc::new(DatabaseManager::new(&db_path).await?);

    // Initialize services
    let query_service = Arc::new(QueryService::with_database(db_manager.clone()));

    // Initialize analytics service if Google AI API key is available
    let analytics_service = match std::env::var(retrochat::env::apis::GOOGLE_AI_API_KEY) {
        Ok(api_key) if !api_key.is_empty() => {
            let google_ai_config = GoogleAiConfig::new(api_key);
            match GoogleAiClient::new(google_ai_config) {
                Ok(client) => Some(Arc::new(AnalyticsRequestService::new(
                    db_manager.clone(),
                    client,
                ))),
                Err(e) => {
                    eprintln!("Warning: Failed to initialize Google AI client: {}", e);
                    None
                }
            }
        }
        _ => None,
    };

    let app_state = Arc::new(Mutex::new(AppState {
        db_manager,
        query_service,
        analytics_service,
    }));

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            get_sessions,
            get_session_detail,
            search_messages,
            get_providers,
            analyze_session,
            create_analysis,
            run_analysis,
            get_analysis_status,
            get_analysis_result,
            list_analyses,
            cancel_analysis,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");

    Ok(())
}
