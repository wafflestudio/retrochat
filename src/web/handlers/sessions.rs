use axum::{
    extract::{Path, Query},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Deserialize;
use std::sync::Arc;

use crate::database::DatabaseManager;
use crate::services::{
    QueryService, SessionDetailRequest, SessionFilters, SessionsQueryRequest,
};

#[derive(Debug, Deserialize)]
pub struct SessionsQuery {
    pub page: Option<i32>,
    pub page_size: Option<i32>,
    pub sort_by: Option<String>,
    pub sort_order: Option<String>,
    pub provider: Option<String>,
    pub project: Option<String>,
}

pub async fn list_sessions(
    Query(params): Query<SessionsQuery>,
) -> Result<Json<serde_json::Value>, AppError> {
    let db_path = crate::database::config::get_default_db_path()
        .map_err(|e| AppError::Internal(format!("Failed to get database path: {e}")))?;

    let db_manager = Arc::new(
        DatabaseManager::new(&db_path)
            .await
            .map_err(|e| AppError::Internal(format!("Failed to connect to database: {e}")))?,
    );

    let service = QueryService::with_database(db_manager);

    let filters = if params.provider.is_some() || params.project.is_some() {
        Some(SessionFilters {
            provider: params.provider,
            project: params.project,
            date_range: None,
            min_messages: None,
            max_messages: None,
        })
    } else {
        None
    };

    let request = SessionsQueryRequest {
        page: params.page,
        page_size: params.page_size,
        sort_by: params.sort_by,
        sort_order: params.sort_order,
        filters,
    };

    let response = service
        .query_sessions(request)
        .await
        .map_err(|e| AppError::Internal(format!("Failed to query sessions: {e}")))?;

    Ok(Json(serde_json::to_value(response).unwrap()))
}

pub async fn get_session_detail(
    Path(session_id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let db_path = crate::database::config::get_default_db_path()
        .map_err(|e| AppError::Internal(format!("Failed to get database path: {e}")))?;

    let db_manager = Arc::new(
        DatabaseManager::new(&db_path)
            .await
            .map_err(|e| AppError::Internal(format!("Failed to connect to database: {e}")))?,
    );

    let service = QueryService::with_database(db_manager);

    let request = SessionDetailRequest {
        session_id: session_id.clone(),
        include_content: Some(true),
        message_limit: None,
        message_offset: None,
    };

    let response = service
        .get_session_detail(request)
        .await
        .map_err(|_e| AppError::NotFound(format!("Session not found: {session_id}")))?;

    Ok(Json(serde_json::to_value(response).unwrap()))
}

// Custom error type for better error handling
pub enum AppError {
    NotFound(String),
    Internal(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
            AppError::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
        };

        let body = Json(serde_json::json!({
            "error": message,
        }));

        (status, body).into_response()
    }
}
