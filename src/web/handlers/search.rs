use axum::{extract::Query, Json};
use serde::Deserialize;
use std::sync::Arc;

use crate::database::DatabaseManager;
use crate::services::{QueryService, SearchRequest};

use super::sessions::AppError;

#[derive(Debug, Deserialize)]
pub struct SearchQuery {
    pub query: String,
    pub page: Option<i32>,
    pub page_size: Option<i32>,
}

pub async fn search_messages(
    Query(params): Query<SearchQuery>,
) -> Result<Json<serde_json::Value>, AppError> {
    let db_path = crate::database::config::get_default_db_path()
        .map_err(|e| AppError::Internal(format!("Failed to get database path: {e}")))?;

    let db_manager = Arc::new(
        DatabaseManager::new(&db_path)
            .await
            .map_err(|e| AppError::Internal(format!("Failed to connect to database: {e}")))?,
    );

    let service = QueryService::with_database(db_manager);

    let request = SearchRequest {
        query: params.query,
        providers: None,
        projects: None,
        date_range: None,
        search_type: None,
        page: params.page,
        page_size: params.page_size,
    };

    let response = service
        .search_messages(request)
        .await
        .map_err(|e| AppError::Internal(format!("Search failed: {e}")))?;

    Ok(Json(serde_json::to_value(response).unwrap()))
}
