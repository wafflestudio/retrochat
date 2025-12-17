//! DTOs (Data Transfer Objects) for Tauri frontend communication.
//!
//! These structs define the API contract between the Tauri backend and the React frontend.
//! They are intentionally separate from the internal database models to allow independent
//! evolution of both layers.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// =============================================================================
// Session DTOs
// =============================================================================

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
    pub raw_input: Option<serde_json::Value>,
    pub raw_result: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FileMetadataItem {
    pub file_path: String,
    pub file_extension: Option<String>,
    pub is_code_file: Option<bool>,
    pub lines_added: Option<i32>,
    pub lines_removed: Option<i32>,
}

// =============================================================================
// Search DTOs
// =============================================================================

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchResultItem {
    pub session_id: String,
    pub message_id: String,
    pub content: String,
    pub role: String,
    pub timestamp: String,
    pub provider: String,
}

// =============================================================================
// Import DTOs
// =============================================================================

#[derive(Debug, Serialize, Deserialize)]
pub struct ImportFileResult {
    pub file_path: String,
    pub sessions_imported: i32,
    pub messages_imported: i32,
    pub success: bool,
    pub error: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ImportSessionsResponse {
    pub total_files: i32,
    pub successful_imports: i32,
    pub failed_imports: i32,
    pub total_sessions_imported: i32,
    pub total_messages_imported: i32,
    pub results: Vec<ImportFileResult>,
}

// =============================================================================
// Analytics Request DTOs
// =============================================================================

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

// =============================================================================
// Analytics Result DTOs
// =============================================================================

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
    pub tool_distribution: HashMap<String, u64>,
    pub average_execution_time_ms: f64,
}

// =============================================================================
// Conversion implementations from domain models to DTOs
// =============================================================================

impl From<retrochat_core::models::Analytics> for AnalyticsItem {
    fn from(analytics: retrochat_core::models::Analytics) -> Self {
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

impl From<retrochat_core::services::analytics::AIQualitativeOutput> for AIQualitativeOutputItem {
    fn from(output: retrochat_core::services::analytics::AIQualitativeOutput) -> Self {
        Self {
            entries: output.entries.into_iter().map(Into::into).collect(),
            summary: output.summary.map(Into::into),
            entries_version: output.entries_version,
        }
    }
}

impl From<retrochat_core::services::analytics::QualitativeEntryOutput>
    for QualitativeEntryOutputItem
{
    fn from(entry: retrochat_core::services::analytics::QualitativeEntryOutput) -> Self {
        Self {
            key: entry.key,
            title: entry.title,
            description: entry.description,
            summary: entry.summary,
            items: entry.items,
        }
    }
}

impl From<retrochat_core::services::analytics::QualitativeEvaluationSummary>
    for QualitativeEvaluationSummaryItem
{
    fn from(summary: retrochat_core::services::analytics::QualitativeEvaluationSummary) -> Self {
        Self {
            total_entries: summary.total_entries,
            categories_evaluated: summary.categories_evaluated,
            entries_version: summary.entries_version,
        }
    }
}

impl From<retrochat_core::services::analytics::AIQuantitativeOutput> for AIQuantitativeOutputItem {
    fn from(output: retrochat_core::services::analytics::AIQuantitativeOutput) -> Self {
        Self {
            rubric_scores: output.rubric_scores.into_iter().map(Into::into).collect(),
            rubric_summary: output.rubric_summary.map(Into::into),
        }
    }
}

impl From<retrochat_core::services::analytics::RubricScore> for RubricScoreItem {
    fn from(score: retrochat_core::services::analytics::RubricScore) -> Self {
        Self {
            rubric_id: score.rubric_id,
            rubric_name: score.rubric_name,
            score: score.score,
            max_score: score.max_score,
            reasoning: score.reasoning,
        }
    }
}

impl From<retrochat_core::services::analytics::RubricEvaluationSummary>
    for RubricEvaluationSummaryItem
{
    fn from(summary: retrochat_core::services::analytics::RubricEvaluationSummary) -> Self {
        Self {
            total_score: summary.total_score,
            max_score: summary.max_score,
            percentage: summary.percentage,
            rubrics_evaluated: summary.rubrics_evaluated,
            rubrics_version: summary.rubrics_version,
        }
    }
}

impl From<retrochat_core::services::analytics::MetricQuantitativeOutput>
    for MetricQuantitativeOutputItem
{
    fn from(output: retrochat_core::services::analytics::MetricQuantitativeOutput) -> Self {
        Self {
            file_changes: output.file_changes.into(),
            time_metrics: output.time_metrics.into(),
            token_metrics: output.token_metrics.into(),
            tool_usage: output.tool_usage.into(),
        }
    }
}

impl From<retrochat_core::services::analytics::FileChangeMetrics> for FileChangeMetricsItem {
    fn from(metrics: retrochat_core::services::analytics::FileChangeMetrics) -> Self {
        Self {
            total_files_modified: metrics.total_files_modified,
            total_files_read: metrics.total_files_read,
            lines_added: metrics.lines_added,
            lines_removed: metrics.lines_removed,
            net_code_growth: metrics.net_code_growth,
        }
    }
}

impl From<retrochat_core::services::analytics::TimeConsumptionMetrics>
    for TimeConsumptionMetricsItem
{
    fn from(metrics: retrochat_core::services::analytics::TimeConsumptionMetrics) -> Self {
        Self {
            total_session_time_minutes: metrics.total_session_time_minutes,
            peak_hours: metrics.peak_hours,
        }
    }
}

impl From<retrochat_core::services::analytics::TokenConsumptionMetrics>
    for TokenConsumptionMetricsItem
{
    fn from(metrics: retrochat_core::services::analytics::TokenConsumptionMetrics) -> Self {
        Self {
            total_tokens_used: metrics.total_tokens_used,
            input_tokens: metrics.input_tokens,
            output_tokens: metrics.output_tokens,
            token_efficiency: metrics.token_efficiency,
        }
    }
}

impl From<retrochat_core::services::analytics::ToolUsageMetrics> for ToolUsageMetricsItem {
    fn from(metrics: retrochat_core::services::analytics::ToolUsageMetrics) -> Self {
        Self {
            total_operations: metrics.total_operations,
            successful_operations: metrics.successful_operations,
            failed_operations: metrics.failed_operations,
            tool_distribution: metrics.tool_distribution,
            average_execution_time_ms: metrics.average_execution_time_ms,
        }
    }
}

// =============================================================================
// Histogram DTOs
// =============================================================================

#[derive(Debug, Serialize, Deserialize)]
pub struct HistogramRequest {
    pub start_time: String,    // RFC3339 timestamp
    pub end_time: String,      // RFC3339 timestamp
    pub interval_minutes: i32, // 5, 15, 60, 360
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HistogramBucket {
    pub timestamp: String, // Bucket start time (RFC3339)
    pub count: i32,        // Count in this bucket
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HistogramResponse {
    pub buckets: Vec<HistogramBucket>,
    pub total_count: i32,
    pub start_time: String,
    pub end_time: String,
    pub interval_minutes: i32,
}
