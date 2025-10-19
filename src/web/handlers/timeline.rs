use axum::{extract::Query, Json};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use std::sync::Arc;

use crate::database::DatabaseManager;
use crate::utils::time_parser;

use super::sessions::AppError;

#[derive(Debug, Deserialize)]
pub struct TimelineQuery {
    pub since: Option<String>,
    pub until: Option<String>,
    pub provider: Option<String>,
    pub role: Option<String>,
    pub reverse: Option<bool>,
    pub format: Option<String>,
}

#[derive(Debug, Serialize, FromRow)]
pub struct TimelineMessage {
    pub timestamp: DateTime<Utc>,
    pub role: String,
    pub provider: String,
    pub project: Option<String>,
    pub session_id: String,
    pub content: String,
    pub message_id: String,
}

#[derive(Debug, Serialize)]
pub struct TimelineResponse {
    pub messages: Vec<TimelineMessage>,
    pub total_count: usize,
    pub time_range: TimeRange,
}

#[derive(Debug, Serialize)]
pub struct TimeRange {
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
}

pub async fn query_timeline(
    Query(params): Query<TimelineQuery>,
) -> Result<Json<TimelineResponse>, AppError> {
    // Parse time specifications
    let from = if let Some(since_str) = params.since {
        Some(
            time_parser::parse_time_spec(&since_str)
                .map_err(|e| AppError::Internal(format!("Invalid 'since' time: {e}")))?,
        )
    } else {
        None
    };

    let to = if let Some(until_str) = params.until {
        Some(
            time_parser::parse_time_spec(&until_str)
                .map_err(|e| AppError::Internal(format!("Invalid 'until' time: {e}")))?,
        )
    } else {
        None
    };

    // Get database
    let db_path = crate::database::config::get_default_db_path()
        .map_err(|e| AppError::Internal(format!("Failed to get database path: {e}")))?;

    let db_manager = Arc::new(
        DatabaseManager::new(&db_path)
            .await
            .map_err(|e| AppError::Internal(format!("Failed to connect to database: {e}")))?,
    );

    // Determine if we should exclude tool messages (compact mode excludes them)
    let format = params.format.as_deref().unwrap_or("compact");
    let exclude_tool_messages = format == "compact";

    // Build SQL query with JOIN (more efficient than N+1 queries)
    let mut sql = String::from(
        r#"
        SELECT
            m.timestamp,
            m.role as role,
            cs.provider as provider,
            cs.project_path as project,
            m.session_id as session_id,
            m.content as content,
            m.id as message_id
        FROM messages m
        JOIN chat_sessions cs ON cs.id = m.session_id
        "#,
    );

    let mut conditions = Vec::new();

    if from.is_some() {
        conditions.push("m.timestamp >= ?");
    }

    if to.is_some() {
        conditions.push("m.timestamp <= ?");
    }

    if params.provider.is_some() {
        conditions.push("cs.provider = ?");
    }

    if params.role.is_some() {
        conditions.push("m.role = ?");
    }

    // Apply tool message filtering for compact mode (same logic as MessageRepository)
    if exclude_tool_messages {
        conditions.push("m.tool_uses IS NULL AND m.tool_results IS NULL");
    }

    if !conditions.is_empty() {
        sql.push_str(" WHERE ");
        sql.push_str(&conditions.join(" AND "));
    }

    sql.push_str(" ORDER BY m.timestamp ");
    sql.push_str(if params.reverse.unwrap_or(false) {
        "DESC"
    } else {
        "ASC"
    });

    let mut query = sqlx::query_as::<_, TimelineMessage>(&sql);

    if let Some(from_time) = from {
        query = query.bind(from_time.to_rfc3339());
    }

    if let Some(to_time) = to {
        query = query.bind(to_time.to_rfc3339());
    }

    if let Some(provider) = &params.provider {
        query = query.bind(provider);
    }

    if let Some(role) = &params.role {
        query = query.bind(role);
    }

    let timeline_messages = query
        .fetch_all(db_manager.pool())
        .await
        .map_err(|e| AppError::Internal(format!("Failed to query timeline: {e}")))?;

    let total_count = timeline_messages.len();

    Ok(Json(TimelineResponse {
        messages: timeline_messages,
        total_count,
        time_range: TimeRange { from, to },
    }))
}
