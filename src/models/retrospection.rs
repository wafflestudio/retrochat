use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::services::analytics::models::{
    ComprehensiveAnalysis, ProcessedQuantitativeOutput, QualitativeInput, QualitativeOutput,
    QuantitativeInput,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Retrospection {
    pub id: String,
    pub retrospect_request_id: String,
    pub generated_at: DateTime<Utc>,

    // Quantitative scores (for queries/filtering)
    pub overall_score: f64,
    pub code_quality_score: f64,
    pub productivity_score: f64,
    pub efficiency_score: f64,
    pub collaboration_score: f64,
    pub learning_score: f64,

    // Key metrics (for queries/aggregation)
    pub total_files_modified: i32,
    pub total_files_read: i32,
    pub lines_added: i32,
    pub lines_removed: i32,
    pub total_tokens_used: i32,
    pub session_duration_minutes: f64,

    // Full analysis data
    pub quantitative_input: QuantitativeInput,
    pub qualitative_input: QualitativeInput,
    pub qualitative_output: QualitativeOutput,
    pub processed_output: ProcessedQuantitativeOutput,

    // Metadata
    pub model_used: Option<String>,
    pub analysis_duration_ms: Option<i64>,
}

impl Retrospection {
    pub fn from_comprehensive_analysis(
        retrospect_request_id: String,
        analysis: ComprehensiveAnalysis,
        model_used: Option<String>,
        analysis_duration_ms: Option<i64>,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            retrospect_request_id,
            generated_at: analysis.generated_at,
            overall_score: analysis.quantitative_output.overall_score,
            code_quality_score: analysis.quantitative_output.code_quality_score,
            productivity_score: analysis.quantitative_output.productivity_score,
            efficiency_score: analysis.quantitative_output.efficiency_score,
            collaboration_score: analysis.quantitative_output.collaboration_score,
            learning_score: analysis.quantitative_output.learning_score,
            total_files_modified: analysis
                .quantitative_input
                .file_changes
                .total_files_modified as i32,
            total_files_read: analysis.quantitative_input.file_changes.total_files_read as i32,
            lines_added: analysis.quantitative_input.file_changes.lines_added as i32,
            lines_removed: analysis.quantitative_input.file_changes.lines_removed as i32,
            total_tokens_used: analysis.quantitative_input.token_metrics.total_tokens_used as i32,
            session_duration_minutes: analysis
                .quantitative_input
                .time_metrics
                .total_session_time_minutes,
            quantitative_input: analysis.quantitative_input,
            qualitative_input: analysis.qualitative_input,
            qualitative_output: analysis.qualitative_output,
            processed_output: analysis.processed_output,
            model_used,
            analysis_duration_ms,
        }
    }

    pub fn to_comprehensive_analysis(&self, session_id: String) -> ComprehensiveAnalysis {
        ComprehensiveAnalysis {
            session_id,
            generated_at: self.generated_at,
            quantitative_input: self.quantitative_input.clone(),
            qualitative_input: self.qualitative_input.clone(),
            quantitative_output: crate::services::analytics::models::QuantitativeOutput {
                overall_score: self.overall_score,
                code_quality_score: self.code_quality_score,
                productivity_score: self.productivity_score,
                efficiency_score: self.efficiency_score,
                collaboration_score: self.collaboration_score,
                learning_score: self.learning_score,
            },
            qualitative_output: self.qualitative_output.clone(),
            processed_output: self.processed_output.clone(),
        }
    }

    // Helper methods to extract simple text for backward compatibility
    pub fn get_insights_text(&self) -> String {
        self.qualitative_output
            .insights
            .iter()
            .map(|i| format!("**{}**: {}", i.title, i.description))
            .collect::<Vec<_>>()
            .join("\n\n")
    }

    pub fn get_recommendations_text(&self) -> String {
        self.qualitative_output
            .recommendations
            .iter()
            .map(|r| format!("**{}**: {}", r.title, r.description))
            .collect::<Vec<_>>()
            .join("\n\n")
    }

    pub fn get_reflection_text(&self) -> String {
        let mut parts = Vec::new();

        if !self.qualitative_output.good_patterns.is_empty() {
            let patterns = self
                .qualitative_output
                .good_patterns
                .iter()
                .map(|p| format!("- {}: {}", p.pattern_name, p.description))
                .collect::<Vec<_>>()
                .join("\n");
            parts.push(format!("**Good Patterns:**\n{patterns}"));
        }

        if !self.qualitative_output.improvement_areas.is_empty() {
            let areas = self
                .qualitative_output
                .improvement_areas
                .iter()
                .map(|a| format!("- {}: {}", a.area_name, a.suggested_improvement))
                .collect::<Vec<_>>()
                .join("\n");
            parts.push(format!("**Areas for Improvement:**\n{areas}"));
        }

        parts.join("\n\n")
    }
}
