// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use retrochat::database::{config, DatabaseManager};
use retrochat::services::{
    QueryService, SearchRequest, SessionDetailRequest, SessionFilters, SessionsQueryRequest,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::State;
use tokio::sync::Mutex;

// Application state
pub struct AppState {
    #[allow(dead_code)]
    db_manager: Arc<DatabaseManager>,
    query_service: Arc<QueryService>,
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
    let state = state.lock().await;

    let request = SessionDetailRequest {
        session_id,
        include_content: Some(true),
        message_limit: None,
        message_offset: None,
    };

    let response = state
        .query_service
        .get_session_detail(request)
        .await
        .map_err(|e| e.to_string())?;

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
            .map(|m| MessageItem {
                id: m.id.to_string(),
                role: m.role.to_string(),
                content: m.content,
                timestamp: m.timestamp.to_rfc3339(),
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

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize database
    let db_path = config::get_default_db_path()?;
    let db_manager = Arc::new(DatabaseManager::new(&db_path).await?);

    // Initialize services
    let query_service = Arc::new(QueryService::with_database(db_manager.clone()));

    let app_state = Arc::new(Mutex::new(AppState {
        db_manager,
        query_service,
    }));

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            get_sessions,
            get_session_detail,
            search_messages,
            get_providers,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");

    Ok(())
}
