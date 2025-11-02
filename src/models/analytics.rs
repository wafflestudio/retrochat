use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// Re-export types from services that will be stored as JSON
use crate::services::analytics::{
    ProcessedQuantitativeOutput, QualitativeInput, QualitativeOutput, QuantitativeInput,
    QuantitativeOutput,
};

// =============================================================================
// Analytics Model (DB representation)
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scores {
    pub overall: f64,
    pub code_quality: f64,
    pub productivity: f64,
    pub efficiency: f64,
    pub collaboration: f64,
    pub learning: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metrics {
    pub total_files_modified: u64,
    pub total_files_read: u64,
    pub lines_added: u64,
    pub lines_removed: u64,
    pub total_tokens_used: u64,
    pub session_duration_minutes: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Analytics {
    pub id: String,
    pub analytics_request_id: String,
    pub session_id: String,
    pub generated_at: DateTime<Utc>,

    // Consolidated JSON groups
    pub scores: Scores,
    pub metrics: Metrics,

    // Complex data structures (stored as JSON strings in DB, deserialized here)
    pub quantitative_input: QuantitativeInput,
    pub qualitative_input: QualitativeInput,
    pub qualitative_output: QualitativeOutput,
    pub processed_output: ProcessedQuantitativeOutput,

    // Metadata
    pub model_used: Option<String>,
    pub analysis_duration_ms: Option<i64>,
}

impl Analytics {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        analytics_request_id: String,
        session_id: String,
        quantitative_input: QuantitativeInput,
        qualitative_input: QualitativeInput,
        quantitative_output: QuantitativeOutput,
        qualitative_output: QualitativeOutput,
        processed_output: ProcessedQuantitativeOutput,
        model_used: Option<String>,
        analysis_duration_ms: Option<i64>,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            analytics_request_id,
            session_id,
            generated_at: Utc::now(),
            scores: Scores {
                overall: quantitative_output.overall_score,
                code_quality: quantitative_output.code_quality_score,
                productivity: quantitative_output.productivity_score,
                efficiency: quantitative_output.efficiency_score,
                collaboration: quantitative_output.collaboration_score,
                learning: quantitative_output.learning_score,
            },
            metrics: Metrics {
                total_files_modified: quantitative_input.file_changes.total_files_modified,
                total_files_read: quantitative_input.file_changes.total_files_read,
                lines_added: quantitative_input.file_changes.lines_added,
                lines_removed: quantitative_input.file_changes.lines_removed,
                total_tokens_used: quantitative_input.token_metrics.total_tokens_used,
                session_duration_minutes: quantitative_input
                    .time_metrics
                    .total_session_time_minutes,
            },
            quantitative_input,
            qualitative_input,
            qualitative_output,
            processed_output,
            model_used,
            analysis_duration_ms,
        }
    }
}
