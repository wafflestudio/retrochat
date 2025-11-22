use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// Re-export types from services that will be stored as JSON
use crate::services::analytics::{
    AIQualitativeOutput, AIQuantitativeOutput, ProcessedQuantitativeOutput, QuantitativeOutput,
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

    pub qualitative_output: AIQualitativeOutput,
    pub processed_output: ProcessedQuantitativeOutput,
    pub ai_quantitative_output: AIQuantitativeOutput,

    // Metadata
    pub model_used: Option<String>,
    pub analysis_duration_ms: Option<i64>,
}

impl Analytics {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        analytics_request_id: String,
        session_id: String,
        quantitative_output: QuantitativeOutput,
        qualitative_output: AIQualitativeOutput,
        processed_output: ProcessedQuantitativeOutput,
        ai_quantitative_output: AIQuantitativeOutput,
        metrics: Metrics,
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
            metrics,
            qualitative_output,
            processed_output,
            ai_quantitative_output,
            model_used,
            analysis_duration_ms,
        }
    }
}
