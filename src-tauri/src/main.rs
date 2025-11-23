// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use retrochat::database::{config, DatabaseManager, ToolOperationRepository};
use retrochat::services::{
    google_ai::{GoogleAiClient, GoogleAiConfig},
    AnalyticsRequestService, QueryService, SearchRequest, SessionDetailRequest, SessionFilters,
    SessionsQueryRequest,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::Manager;
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
    pub created_by: Option<String>,
    pub error_message: Option<String>,
}

// Analytics Result DTOs
#[derive(Debug, Serialize, Deserialize)]
pub struct AnalyticsItem {
    pub id: String,
    pub analytics_request_id: String,
    pub session_id: String,
    pub generated_at: String,
    pub ai_qualitative_output: AIQualitativeOutputItem,
    pub ai_quantitative_output: AIQuantitativeOutputItem,
    pub metric_quantitative_output: MetricQuantitativeOutputItem,
    pub model_used: Option<String>,
    pub analysis_duration_ms: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AIQualitativeOutputItem {
    pub entries: Vec<QualitativeEntryOutputItem>,
    pub summary: Option<QualitativeEvaluationSummaryItem>,
    pub entries_version: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct QualitativeEntryOutputItem {
    pub key: String,
    pub title: String,
    pub description: String,
    pub summary: String,
    pub items: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct QualitativeEvaluationSummaryItem {
    pub total_entries: usize,
    pub categories_evaluated: usize,
    pub entries_version: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AIQuantitativeOutputItem {
    pub rubric_scores: Vec<RubricScoreItem>,
    pub rubric_summary: Option<RubricEvaluationSummaryItem>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RubricScoreItem {
    pub rubric_id: String,
    pub rubric_name: String,
    pub score: f64,
    pub max_score: f64,
    pub reasoning: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RubricEvaluationSummaryItem {
    pub total_score: f64,
    pub max_score: f64,
    pub percentage: f64,
    pub rubrics_evaluated: usize,
    pub rubrics_version: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MetricQuantitativeOutputItem {
    pub file_changes: FileChangeMetricsItem,
    pub time_metrics: TimeConsumptionMetricsItem,
    pub token_metrics: TokenConsumptionMetricsItem,
    pub tool_usage: ToolUsageMetricsItem,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FileChangeMetricsItem {
    pub total_files_modified: u64,
    pub total_files_read: u64,
    pub lines_added: u64,
    pub lines_removed: u64,
    pub net_code_growth: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TimeConsumptionMetricsItem {
    pub total_session_time_minutes: f64,
    pub peak_hours: Vec<u32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TokenConsumptionMetricsItem {
    pub total_tokens_used: u64,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub token_efficiency: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ToolUsageMetricsItem {
    pub total_operations: u64,
    pub successful_operations: u64,
    pub failed_operations: u64,
    pub tool_distribution: std::collections::HashMap<String, u64>,
    pub average_execution_time_ms: f64,
}

// Conversion implementations
impl From<retrochat::models::Analytics> for AnalyticsItem {
    fn from(analytics: retrochat::models::Analytics) -> Self {
        Self {
            id: analytics.id,
            analytics_request_id: analytics.analytics_request_id,
            session_id: analytics.session_id,
            generated_at: analytics.generated_at.to_rfc3339(),
            ai_qualitative_output: analytics.ai_qualitative_output.into(),
            ai_quantitative_output: analytics.ai_quantitative_output.into(),
            metric_quantitative_output: analytics.metric_quantitative_output.into(),
            model_used: analytics.model_used,
            analysis_duration_ms: analytics.analysis_duration_ms,
        }
    }
}

impl From<retrochat::services::analytics::AIQualitativeOutput> for AIQualitativeOutputItem {
    fn from(output: retrochat::services::analytics::AIQualitativeOutput) -> Self {
        Self {
            entries: output.entries.into_iter().map(Into::into).collect(),
            summary: output.summary.map(Into::into),
            entries_version: output.entries_version,
        }
    }
}

impl From<retrochat::services::analytics::QualitativeEntryOutput> for QualitativeEntryOutputItem {
    fn from(entry: retrochat::services::analytics::QualitativeEntryOutput) -> Self {
        Self {
            key: entry.key,
            title: entry.title,
            description: entry.description,
            summary: entry.summary,
            items: entry.items,
        }
    }
}

impl From<retrochat::services::analytics::QualitativeEvaluationSummary>
    for QualitativeEvaluationSummaryItem
{
    fn from(summary: retrochat::services::analytics::QualitativeEvaluationSummary) -> Self {
        Self {
            total_entries: summary.total_entries,
            categories_evaluated: summary.categories_evaluated,
            entries_version: summary.entries_version,
        }
    }
}

impl From<retrochat::services::analytics::AIQuantitativeOutput> for AIQuantitativeOutputItem {
    fn from(output: retrochat::services::analytics::AIQuantitativeOutput) -> Self {
        Self {
            rubric_scores: output.rubric_scores.into_iter().map(Into::into).collect(),
            rubric_summary: output.rubric_summary.map(Into::into),
        }
    }
}

impl From<retrochat::services::analytics::RubricScore> for RubricScoreItem {
    fn from(score: retrochat::services::analytics::RubricScore) -> Self {
        Self {
            rubric_id: score.rubric_id,
            rubric_name: score.rubric_name,
            score: score.score,
            max_score: score.max_score,
            reasoning: score.reasoning,
        }
    }
}

impl From<retrochat::services::analytics::RubricEvaluationSummary> for RubricEvaluationSummaryItem {
    fn from(summary: retrochat::services::analytics::RubricEvaluationSummary) -> Self {
        Self {
            total_score: summary.total_score,
            max_score: summary.max_score,
            percentage: summary.percentage,
            rubrics_evaluated: summary.rubrics_evaluated,
            rubrics_version: summary.rubrics_version,
        }
    }
}

impl From<retrochat::services::analytics::MetricQuantitativeOutput>
    for MetricQuantitativeOutputItem
{
    fn from(output: retrochat::services::analytics::MetricQuantitativeOutput) -> Self {
        Self {
            file_changes: output.file_changes.into(),
            time_metrics: output.time_metrics.into(),
            token_metrics: output.token_metrics.into(),
            tool_usage: output.tool_usage.into(),
        }
    }
}

impl From<retrochat::services::analytics::FileChangeMetrics> for FileChangeMetricsItem {
    fn from(metrics: retrochat::services::analytics::FileChangeMetrics) -> Self {
        Self {
            total_files_modified: metrics.total_files_modified,
            total_files_read: metrics.total_files_read,
            lines_added: metrics.lines_added,
            lines_removed: metrics.lines_removed,
            net_code_growth: metrics.net_code_growth,
        }
    }
}

impl From<retrochat::services::analytics::TimeConsumptionMetrics> for TimeConsumptionMetricsItem {
    fn from(metrics: retrochat::services::analytics::TimeConsumptionMetrics) -> Self {
        Self {
            total_session_time_minutes: metrics.total_session_time_minutes,
            peak_hours: metrics.peak_hours,
        }
    }
}

impl From<retrochat::services::analytics::TokenConsumptionMetrics> for TokenConsumptionMetricsItem {
    fn from(metrics: retrochat::services::analytics::TokenConsumptionMetrics) -> Self {
        Self {
            total_tokens_used: metrics.total_tokens_used,
            input_tokens: metrics.input_tokens,
            output_tokens: metrics.output_tokens,
            token_efficiency: metrics.token_efficiency,
        }
    }
}

impl From<retrochat::services::analytics::ToolUsageMetrics> for ToolUsageMetricsItem {
    fn from(metrics: retrochat::services::analytics::ToolUsageMetrics) -> Self {
        Self {
            total_operations: metrics.total_operations,
            successful_operations: metrics.successful_operations,
            failed_operations: metrics.failed_operations,
            tool_distribution: metrics.tool_distribution,
            average_execution_time_ms: metrics.average_execution_time_ms,
        }
    }
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
        created_by: completed_request.created_by,
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
        created_by: request.created_by,
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
        created_by: request.created_by,
        error_message: request.error_message,
    })
}

#[tauri::command]
async fn get_analysis_result(
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
            created_by: r.created_by,
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
        .setup(|app| {
            #[cfg(debug_assertions)]
            {
                let window = app.get_webview_window("main").unwrap();
                window.open_devtools();
                window.close_devtools();
            }
            Ok(())
        })
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
