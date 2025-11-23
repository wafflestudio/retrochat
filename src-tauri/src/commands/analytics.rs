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
    log::info!(
        "analyze_session called - session_id: {}, custom_prompt: {:?}",
        session_id,
        custom_prompt
            .as_ref()
            .map(|p| format!("{}...", &p.chars().take(50).collect::<String>()))
    );

    let state_guard = state.lock().await;

    let analytics_service = state_guard.analytics_service.as_ref().ok_or_else(|| {
        log::error!("Analytics service not available");
        "Analytics service not available. Please set GOOGLE_AI_API_KEY environment variable."
            .to_string()
    })?;

    // Create the request
    log::debug!("Creating analysis request");
    let request = analytics_service
        .create_analysis_request(session_id.clone(), None, custom_prompt)
        .await
        .map_err(|e| {
            log::error!("Failed to create analysis request: {}", e);
            e.to_string()
        })?;

    let request_id = request.id.clone();
    log::info!("Created analysis request with ID: {}", request_id);

    // Execute immediately
    log::debug!("Executing analysis");
    analytics_service
        .execute_analysis(request_id.clone())
        .await
        .map_err(|e| {
            log::error!("Failed to execute analysis: {}", e);
            e.to_string()
        })?;

    // Get the updated status
    log::debug!("Fetching completed analysis status");
    let completed_request = analytics_service
        .get_analysis_status(request_id.clone())
        .await
        .map_err(|e| {
            log::error!("Failed to get analysis status: {}", e);
            e.to_string()
        })?;

    log::info!(
        "Analysis completed successfully - status: {}",
        completed_request.status
    );

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
    log::info!(
        "create_analysis called - session_id: {}, custom_prompt: {:?}",
        session_id,
        custom_prompt
            .as_ref()
            .map(|p| format!("{}...", &p.chars().take(50).collect::<String>()))
    );

    let state_guard = state.lock().await;

    let analytics_service = state_guard.analytics_service.as_ref().ok_or_else(|| {
        log::error!("Analytics service not available");
        "Analytics service not available. Please set GOOGLE_AI_API_KEY environment variable."
            .to_string()
    })?;

    log::debug!("Creating analysis request");
    let request = analytics_service
        .create_analysis_request(session_id, None, custom_prompt)
        .await
        .map_err(|e| {
            log::error!("Failed to create analysis request: {}", e);
            e.to_string()
        })?;

    log::info!("Successfully created analysis request: {}", request.id);

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
    log::info!("run_analysis called - request_id: {}", request_id);

    let state_guard = state.lock().await;

    let analytics_service = state_guard.analytics_service.as_ref().ok_or_else(|| {
        log::error!("Analytics service not available");
        "Analytics service not available".to_string()
    })?;

    log::debug!("Executing analysis");
    analytics_service
        .execute_analysis(request_id.clone())
        .await
        .map_err(|e| {
            log::error!("Failed to execute analysis: {}", e);
            e.to_string()
        })
        .inspect(|_| {
            log::info!("Analysis execution completed successfully");
        })
}

#[tauri::command]
pub async fn get_analysis_status(
    state: State<'_, Arc<Mutex<AppState>>>,
    request_id: String,
) -> Result<AnalyticsRequestItem, String> {
    log::debug!("get_analysis_status called - request_id: {}", request_id);

    let state_guard = state.lock().await;

    let analytics_service = state_guard.analytics_service.as_ref().ok_or_else(|| {
        log::error!("Analytics service not available");
        "Analytics service not available".to_string()
    })?;

    let request = analytics_service
        .get_analysis_status(request_id.clone())
        .await
        .map_err(|e| {
            log::error!("Failed to get analysis status: {}", e);
            e.to_string()
        })?;

    log::debug!("Analysis status: {}", request.status);

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
    log::debug!("get_analysis_result called - request_id: {}", request_id);

    let state_guard = state.lock().await;

    let analytics_service = state_guard.analytics_service.as_ref().ok_or_else(|| {
        log::error!("Analytics service not available");
        "Analytics service not available".to_string()
    })?;

    let result = analytics_service
        .get_analysis_result(request_id.clone())
        .await
        .map_err(|e| {
            log::error!("Failed to get analysis result: {}", e);
            e.to_string()
        })?;

    if result.is_some() {
        log::debug!("Analysis result found");
    } else {
        log::debug!("No analysis result found");
    }

    // Convert Analytics DB entity to Tauri DTO
    Ok(result.map(AnalyticsItem::from))
}

#[tauri::command]
pub async fn list_analyses(
    state: State<'_, Arc<Mutex<AppState>>>,
    session_id: Option<String>,
    limit: Option<usize>,
) -> Result<Vec<AnalyticsRequestItem>, String> {
    log::info!(
        "list_analyses called - session_id: {:?}, limit: {:?}",
        session_id,
        limit
    );

    let state_guard = state.lock().await;

    let analytics_service = state_guard.analytics_service.as_ref().ok_or_else(|| {
        log::error!("Analytics service not available");
        "Analytics service not available".to_string()
    })?;

    let requests = analytics_service
        .list_analyses(session_id, limit)
        .await
        .map_err(|e| {
            log::error!("Failed to list analyses: {}", e);
            e.to_string()
        })?;

    log::info!(
        "Successfully retrieved {} analysis requests",
        requests.len()
    );

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
    log::info!("cancel_analysis called - request_id: {}", request_id);

    let state_guard = state.lock().await;

    let analytics_service = state_guard.analytics_service.as_ref().ok_or_else(|| {
        log::error!("Analytics service not available");
        "Analytics service not available".to_string()
    })?;

    analytics_service
        .cancel_analysis(request_id.clone())
        .await
        .map_err(|e| {
            log::error!("Failed to cancel analysis: {}", e);
            e.to_string()
        })
        .inspect(|_| {
            log::info!("Analysis cancelled successfully");
        })
}
