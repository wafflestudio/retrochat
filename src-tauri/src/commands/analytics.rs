use crate::dto::{AnalyticsItem, AnalyticsRequestItem};
use crate::AppState;
use std::sync::Arc;
use tauri::State;
use tokio::sync::Mutex;

/// Convenience command: Create and execute analysis in one call
#[tauri::command]
pub async fn analyze_session(
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
        created_by: completed_request.created_by,
        error_message: completed_request.error_message,
    })
}

/// Create an analysis request without executing it (for advanced use cases)
#[tauri::command]
pub async fn create_analysis(
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
        created_by: request.created_by,
        error_message: request.error_message,
    })
}

#[tauri::command]
pub async fn run_analysis(
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
pub async fn get_analysis_status(
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
        created_by: request.created_by,
        error_message: request.error_message,
    })
}

#[tauri::command]
pub async fn get_analysis_result(
    state: State<'_, Arc<Mutex<AppState>>>,
    request_id: String,
) -> Result<Option<AnalyticsItem>, String> {
    let state_guard = state.lock().await;

    let analytics_service = state_guard
        .analytics_service
        .as_ref()
        .ok_or("Analytics service not available")?;

    let result = analytics_service
        .get_analysis_result(request_id)
        .await
        .map_err(|e| e.to_string())?;

    // Convert Analytics DB entity to Tauri DTO
    Ok(result.map(AnalyticsItem::from))
}

#[tauri::command]
pub async fn list_analyses(
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
            created_by: r.created_by,
            error_message: r.error_message,
        })
        .collect())
}

#[tauri::command]
pub async fn cancel_analysis(
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
