use chrono::DateTime;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::services::query_service::DateRange;

// =============================================================================
// Basic Analytics Models (기존 구조 유지)
// =============================================================================

#[derive(Debug, Serialize, Deserialize)]
pub struct UsageInsights {
    pub total_sessions: u64,
    pub total_messages: u64,
    pub total_tokens: u64,
    pub date_range: DateRange,
    pub span_days: i64,
    pub provider_breakdown: HashMap<String, ProviderStats>,
    pub daily_activity: Vec<DailyActivity>,
    pub message_role_distribution: MessageRoleDistribution,
    pub top_projects: Vec<ProjectStats>,
    pub session_duration_stats: DurationStats,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProviderStats {
    pub sessions: u64,
    pub messages: u64,
    pub tokens: u64,
    pub percentage_of_total: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DailyActivity {
    pub date: String,
    pub sessions: u64,
    pub messages: u64,
    pub tokens: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MessageRoleDistribution {
    pub user_messages: u64,
    pub assistant_messages: u64,
    pub system_messages: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProjectStats {
    pub project_name: String,
    pub sessions: u64,
    pub messages: u64,
    pub tokens: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DurationStats {
    pub average_minutes: f64,
    pub median_minutes: f64,
    pub min_minutes: f64,
    pub max_minutes: f64,
}

// =============================================================================
// Advanced Analytics Models (새로운 구조)
// =============================================================================

#[derive(Debug, Serialize, Deserialize)]
pub struct ComprehensiveAnalysis {
    pub session_id: String,
    pub generated_at: DateTime<Utc>,
    pub quantitative_input: QuantitativeInput,
    pub qualitative_input: QualitativeInput,
    pub quantitative_output: QuantitativeOutput,
    pub qualitative_output: QualitativeOutput,
    pub processed_output: ProcessedQuantitativeOutput,
}

// =============================================================================
// Quantitative Input Models
// =============================================================================

#[derive(Debug, Serialize, Deserialize)]
pub struct QuantitativeInput {
    pub file_changes: FileChangeMetrics,
    pub time_metrics: TimeConsumptionMetrics,
    pub token_metrics: TokenConsumptionMetrics,
    pub tool_usage: ToolUsageMetrics,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FileChangeMetrics {
    pub total_files_modified: u64,
    pub total_files_read: u64,
    pub lines_added: u64,
    pub lines_removed: u64,
    pub net_code_growth: i64,
    pub refactoring_operations: u64,
    pub bulk_edit_operations: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TimeConsumptionMetrics {
    pub total_session_time_minutes: f64,
    pub average_session_length_minutes: f64,
    pub peak_hours: Vec<u32>,
    pub break_duration_minutes: f64,
    pub context_switching_time_minutes: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TokenConsumptionMetrics {
    pub total_tokens_used: u64,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub token_efficiency: f64,
    pub tokens_per_hour: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ToolUsageMetrics {
    pub total_operations: u64,
    pub successful_operations: u64,
    pub failed_operations: u64,
    pub tool_distribution: HashMap<String, u64>,
    pub average_execution_time_ms: f64,
}

// =============================================================================
// Qualitative Input Models
// =============================================================================

#[derive(Debug, Serialize, Deserialize)]
pub struct QualitativeInput {
    pub file_contexts: Vec<FileContext>,
    pub chat_context: ChatContext,
    pub project_context: ProjectContext,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FileContext {
    pub file_path: String,
    pub file_type: String,
    pub modification_type: String,
    pub content_snippet: String,
    pub complexity_indicators: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChatContext {
    pub conversation_flow: String,
    pub problem_solving_patterns: Vec<String>,
    pub ai_interaction_quality: f64,
    pub key_topics: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProjectContext {
    pub project_type: String,
    pub technology_stack: Vec<String>,
    pub project_complexity: f64,
    pub development_stage: String,
}

// =============================================================================
// Quantitative Output Models
// =============================================================================

#[derive(Debug, Serialize, Deserialize)]
pub struct QuantitativeOutput {
    pub overall_score: f64,
    pub code_quality_score: f64,
    pub productivity_score: f64,
    pub efficiency_score: f64,
    pub collaboration_score: f64,
    pub learning_score: f64,
}

// =============================================================================
// Qualitative Output Models
// =============================================================================

#[derive(Debug, Serialize, Deserialize)]
pub struct QualitativeOutput {
    pub insights: Vec<Insight>,
    pub good_patterns: Vec<GoodPattern>,
    pub improvement_areas: Vec<ImprovementArea>,
    pub recommendations: Vec<Recommendation>,
    pub learning_observations: Vec<LearningObservation>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Insight {
    pub title: String,
    pub description: String,
    pub category: String,
    pub confidence: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GoodPattern {
    pub pattern_name: String,
    pub description: String,
    pub frequency: u64,
    pub impact: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ImprovementArea {
    pub area_name: String,
    pub current_state: String,
    pub suggested_improvement: String,
    pub expected_impact: String,
    pub priority: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Recommendation {
    pub title: String,
    pub description: String,
    pub impact_score: f64,
    pub implementation_difficulty: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LearningObservation {
    pub observation: String,
    pub skill_area: String,
    pub progress_indicator: String,
    pub next_steps: Vec<String>,
}

// =============================================================================
// Processed Quantitative Output Models
// =============================================================================

#[derive(Debug, Serialize, Deserialize)]
pub struct ProcessedQuantitativeOutput {
    pub session_metrics: SessionMetrics,
    pub token_metrics: ProcessedTokenMetrics,
    pub code_change_metrics: ProcessedCodeMetrics,
    pub time_efficiency_metrics: TimeEfficiencyMetrics,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SessionMetrics {
    pub total_sessions: u64,
    pub average_session_duration_minutes: f64,
    pub session_consistency_score: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProcessedTokenMetrics {
    pub total_tokens: u64,
    pub tokens_per_hour: f64,
    pub input_output_ratio: f64,
    pub token_efficiency_score: f64,
    pub cost_estimate: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProcessedCodeMetrics {
    pub net_lines_changed: i64,
    pub files_per_session: f64,
    pub lines_per_hour: f64,
    pub refactoring_ratio: f64,
    pub code_velocity: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TimeEfficiencyMetrics {
    pub productivity_score: f64,
    pub context_switching_cost: f64,
    pub deep_work_ratio: f64,
    pub time_utilization: f64,
}
