use crate::dto::{HistogramBucket, HistogramRequest, HistogramResponse};
use crate::AppState;
use chrono::{DateTime, Utc};
use retrochat_core::database::{ChatSessionRepository, MessageRepository};
use std::sync::Arc;
use tauri::State;
use tokio::sync::Mutex;

#[tauri::command]
pub async fn get_session_activity_histogram(
    state: State<'_, Arc<Mutex<AppState>>>,
    request: HistogramRequest,
) -> Result<HistogramResponse, String> {
    log::info!(
        "get_session_activity_histogram called - start: {}, end: {}, interval: {}",
        request.start_time,
        request.end_time,
        request.interval_minutes
    );

    // Parse timestamps
    let start = DateTime::parse_from_rfc3339(&request.start_time)
        .map_err(|e| {
            log::error!("Invalid start_time format: {}", e);
            format!("Invalid start_time format: {}", e)
        })?
        .with_timezone(&Utc);

    let end = DateTime::parse_from_rfc3339(&request.end_time)
        .map_err(|e| {
            log::error!("Invalid end_time format: {}", e);
            format!("Invalid end_time format: {}", e)
        })?
        .with_timezone(&Utc);

    // Get repository and fetch histogram data
    let state = state.lock().await;
    let session_repo = ChatSessionRepository::new(&state.db_manager);

    let buckets = session_repo
        .get_active_sessions_histogram(&start, &end, request.interval_minutes)
        .await
        .map_err(|e| {
            log::error!("Failed to fetch session activity histogram: {}", e);
            e.to_string()
        })?;

    let total_count: i32 = buckets.iter().map(|(_, count)| count).sum();

    log::info!(
        "Session histogram: {} buckets (total: {}), first: {:?}, last: {:?}",
        buckets.len(),
        total_count,
        buckets.first().map(|(ts, c)| format!("{} ({})", ts, c)),
        buckets.last().map(|(ts, c)| format!("{} ({})", ts, c))
    );

    Ok(HistogramResponse {
        buckets: buckets
            .into_iter()
            .map(|(timestamp, count)| HistogramBucket { timestamp, count })
            .collect(),
        total_count,
        start_time: request.start_time,
        end_time: request.end_time,
        interval_minutes: request.interval_minutes,
    })
}

#[tauri::command]
pub async fn get_user_message_histogram(
    state: State<'_, Arc<Mutex<AppState>>>,
    request: HistogramRequest,
) -> Result<HistogramResponse, String> {
    log::info!(
        "get_user_message_histogram called - start: {}, end: {}, interval: {}",
        request.start_time,
        request.end_time,
        request.interval_minutes
    );

    // Parse timestamps
    let start = DateTime::parse_from_rfc3339(&request.start_time)
        .map_err(|e| {
            log::error!("Invalid start_time format: {}", e);
            format!("Invalid start_time format: {}", e)
        })?
        .with_timezone(&Utc);

    let end = DateTime::parse_from_rfc3339(&request.end_time)
        .map_err(|e| {
            log::error!("Invalid end_time format: {}", e);
            format!("Invalid end_time format: {}", e)
        })?
        .with_timezone(&Utc);

    // Get repository and fetch histogram data
    let state = state.lock().await;
    let message_repo = MessageRepository::new(&state.db_manager);

    let buckets = message_repo
        .get_user_message_histogram(&start, &end, request.interval_minutes)
        .await
        .map_err(|e| {
            log::error!("Failed to fetch user message histogram: {}", e);
            e.to_string()
        })?;

    let total_count: i32 = buckets.iter().map(|(_, count)| count).sum();

    log::info!(
        "Message histogram: {} buckets (total: {}), first: {:?}, last: {:?}",
        buckets.len(),
        total_count,
        buckets.first().map(|(ts, c)| format!("{} ({})", ts, c)),
        buckets.last().map(|(ts, c)| format!("{} ({})", ts, c))
    );

    Ok(HistogramResponse {
        buckets: buckets
            .into_iter()
            .map(|(timestamp, count)| HistogramBucket { timestamp, count })
            .collect(),
        total_count,
        start_time: request.start_time,
        end_time: request.end_time,
        interval_minutes: request.interval_minutes,
    })
}
