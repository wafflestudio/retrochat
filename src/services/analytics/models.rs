use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

// =============================================================================
// Rubric Models (for LLM-as-a-judge evaluation)
// =============================================================================

/// A single evaluation rubric defining criteria for judging user behavior
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rubric {
    /// Unique identifier for the rubric (e.g., "rubric_001")
    pub id: String,
    /// Short descriptive name (2-5 words)
    pub name: String,
    /// What this rubric measures (1-2 sentences)
    pub description: String,
    /// How to score from 1 (poor) to 5 (excellent)
    pub scoring_criteria: String,
    /// Weight for scoring aggregation (default 1.0)
    #[serde(default = "default_weight")]
    pub weight: f64,
}

fn default_weight() -> f64 {
    1.0
}

impl Rubric {
    /// Format rubric for inclusion in LLM prompts
    pub fn format_for_prompt(&self) -> String {
        format!(
            "Name: {}\nDescription: {}\nScoring Criteria:\n{}",
            self.name, self.description, self.scoring_criteria
        )
    }
}

/// Container for a list of rubrics with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RubricList {
    /// Schema version
    #[serde(default = "default_version")]
    pub version: String,
    /// List of rubrics
    pub rubrics: Vec<Rubric>,
}

fn default_version() -> String {
    "1.0".to_string()
}

impl RubricList {
    /// Load rubrics from a JSON file
    pub fn from_json_file(path: &Path) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let rubric_list: RubricList = serde_json::from_str(&content)?;
        Ok(rubric_list)
    }

    /// Load rubrics from embedded JSON string
    pub fn from_json_str(json: &str) -> anyhow::Result<Self> {
        let rubric_list: RubricList = serde_json::from_str(json)?;
        Ok(rubric_list)
    }

    /// Get default rubrics (embedded in binary)
    pub fn default_rubrics() -> Self {
        let json = include_str!("../../../resources/rubrics.json");
        Self::from_json_str(json).expect("Default rubrics should be valid JSON")
    }
}

/// Score for a single rubric evaluation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RubricScore {
    /// ID of the rubric being scored
    pub rubric_id: String,
    /// Name of the rubric for display
    pub rubric_name: String,
    /// Score (1-5 scale)
    pub score: f64,
    /// Maximum possible score (typically 5.0)
    pub max_score: f64,
    /// LLM's reasoning for the score
    pub reasoning: String,
}

impl RubricScore {
    /// Calculate percentage score
    pub fn percentage(&self) -> f64 {
        if self.max_score > 0.0 {
            (self.score / self.max_score) * 100.0
        } else {
            0.0
        }
    }
}

/// Summary of all rubric evaluations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RubricEvaluationSummary {
    /// Total score across all rubrics
    pub total_score: f64,
    /// Maximum possible score
    pub max_score: f64,
    /// Percentage (0-100)
    pub percentage: f64,
    /// Number of rubrics evaluated
    pub rubrics_evaluated: usize,
    /// Version of rubrics used
    pub rubrics_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuantitativeInput {
    pub file_changes: FileChangeMetrics,
    pub time_metrics: TimeConsumptionMetrics,
    pub token_metrics: TokenConsumptionMetrics,
    pub tool_usage: ToolUsageMetrics,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileChangeMetrics {
    pub total_files_modified: u64,
    pub total_files_read: u64,
    pub lines_added: u64,
    pub lines_removed: u64,
    pub net_code_growth: i64,
    pub refactoring_operations: u64,
    pub bulk_edit_operations: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeConsumptionMetrics {
    pub total_session_time_minutes: f64,
    pub average_session_length_minutes: f64,
    pub peak_hours: Vec<u32>,
    pub break_duration_minutes: f64,
    pub context_switching_time_minutes: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenConsumptionMetrics {
    pub total_tokens_used: u64,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub token_efficiency: f64,
    pub tokens_per_hour: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

/// QualitativeInput contains a single raw JSON string representing the full chat session.
/// The JSON includes multi-turn messages with all tool uses embedded in each corresponding message.
/// Long tool content is truncated by cutting the center portion to meet character thresholds.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualitativeInput {
    /// Raw JSON string containing the full session transcript with embedded tool uses.
    /// This is the primary input for qualitative analysis by LLM.
    pub raw_session: String,
}

/// Represents a single turn in the session transcript for JSON serialization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionTurn {
    /// Turn number in the conversation
    pub turn_number: u32,
    /// Message role: "user", "assistant", or "system"
    pub role: String,
    /// The text content of the message
    pub content: String,
}

/// Full session transcript structure for JSON serialization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionTranscript {
    /// Session identifier
    pub session_id: String,
    /// Total number of turns
    pub total_turns: u32,
    /// All turns in the session
    pub turns: Vec<SessionTurn>,
}

// =============================================================================
// Quantitative Output Models
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualitativeOutput {
    pub insights: Vec<Insight>,
    pub good_patterns: Vec<GoodPattern>,
    pub improvement_areas: Vec<ImprovementArea>,
    pub recommendations: Vec<Recommendation>,
    pub learning_observations: Vec<LearningObservation>,
    /// Rubric-based evaluation scores (LLM-as-a-judge)
    #[serde(default)]
    pub rubric_scores: Vec<RubricScore>,
    /// Summary of rubric evaluation
    #[serde(default)]
    pub rubric_summary: Option<RubricEvaluationSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Insight {
    pub title: String,
    pub description: String,
    pub category: String,
    pub confidence: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoodPattern {
    pub pattern_name: String,
    pub description: String,
    pub frequency: u64,
    pub impact: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImprovementArea {
    pub area_name: String,
    pub current_state: String,
    pub suggested_improvement: String,
    pub expected_impact: String,
    pub priority: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Recommendation {
    pub title: String,
    pub description: String,
    pub impact_score: f64,
    pub implementation_difficulty: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningObservation {
    pub observation: String,
    pub skill_area: String,
    pub progress_indicator: String,
    pub next_steps: Vec<String>,
}

// =============================================================================
// Processed Quantitative Output Models
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessedQuantitativeOutput {
    pub session_metrics: SessionMetrics,
    pub token_metrics: ProcessedTokenMetrics,
    pub code_change_metrics: ProcessedCodeMetrics,
    pub time_efficiency_metrics: TimeEfficiencyMetrics,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMetrics {
    pub total_sessions: u64,
    pub average_session_duration_minutes: f64,
    pub session_consistency_score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessedTokenMetrics {
    pub total_tokens: u64,
    pub tokens_per_hour: f64,
    pub input_output_ratio: f64,
    pub token_efficiency_score: f64,
    pub cost_estimate: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessedCodeMetrics {
    pub net_lines_changed: i64,
    pub files_per_session: f64,
    pub lines_per_hour: f64,
    pub refactoring_ratio: f64,
    pub code_velocity: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeEfficiencyMetrics {
    pub productivity_score: f64,
    pub context_switching_cost: f64,
    pub deep_work_ratio: f64,
    pub time_utilization: f64,
}
