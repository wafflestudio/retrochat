use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// Re-export types from services that will be stored as JSON
use crate::services::analytics::{AIQualitativeOutput, AIQuantitativeOutput};

// =============================================================================
// Analytics Model (DB representation)
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Analytics {
    pub id: String,
    pub analytics_request_id: String,
    pub session_id: String,
    pub generated_at: DateTime<Utc>,

    pub ai_qualitative_output: AIQualitativeOutput,
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
        qualitative_output: AIQualitativeOutput,
        ai_quantitative_output: AIQuantitativeOutput,
        model_used: Option<String>,
        analysis_duration_ms: Option<i64>,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            analytics_request_id,
            session_id,
            generated_at: Utc::now(),
            ai_qualitative_output: qualitative_output,
            ai_quantitative_output,
            model_used,
            analysis_duration_ms,
        }
    }
}
