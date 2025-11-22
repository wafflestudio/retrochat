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

// =============================================================================
// Qualitative Entry Models (for configurable qualitative analysis)
// =============================================================================

/// A single qualitative entry definition for configurable analysis output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualitativeEntry {
    /// Unique key for the entry (e.g., "insights", "good_patterns")
    pub key: String,
    /// Display title (e.g., "Insights", "Good Patterns")
    pub title: String,
    /// What this entry measures (1-2 sentences for LLM prompt)
    pub description: String,
}

impl QualitativeEntry {
    /// Format entry for inclusion in LLM prompts
    pub fn format_for_prompt(&self) -> String {
        format!("**{}**: {}", self.title, self.description)
    }
}

/// Container for a list of qualitative entries with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualitativeEntryList {
    /// Schema version
    #[serde(default = "default_version")]
    pub version: String,
    /// List of qualitative entries
    pub entries: Vec<QualitativeEntry>,
}

impl QualitativeEntryList {
    /// Load entries from a JSON file
    pub fn from_json_file(path: &Path) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let entry_list: QualitativeEntryList = serde_json::from_str(&content)?;
        Ok(entry_list)
    }

    /// Load entries from embedded JSON string
    pub fn from_json_str(json: &str) -> anyhow::Result<Self> {
        let entry_list: QualitativeEntryList = serde_json::from_str(json)?;
        Ok(entry_list)
    }

    /// Get default entries (embedded in binary)
    pub fn default_entries() -> Self {
        let json = include_str!("../../../resources/qualitative_entries.json");
        Self::from_json_str(json).expect("Default qualitative entries should be valid JSON")
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeConsumptionMetrics {
    pub total_session_time_minutes: f64,
    pub peak_hours: Vec<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenConsumptionMetrics {
    pub total_tokens_used: u64,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub token_efficiency: f64,
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
// AI Qualitative Output Models (configurable LLM-based qualitative analysis)
// =============================================================================

/// AI-generated qualitative output from configurable entry-based analysis
/// Each entry contains a list of markdown strings (one insight/observation per line)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AIQualitativeOutput {
    /// Dynamic entries based on qualitative_entries.json configuration
    /// Key is the entry key (e.g., "insights"), value is an array of markdown strings
    #[serde(default)]
    pub entries: HashMap<String, Vec<String>>,
    /// Summary of qualitative evaluation
    #[serde(default)]
    pub summary: Option<QualitativeEvaluationSummary>,
    /// Version of qualitative entries configuration used
    #[serde(default)]
    pub entries_version: Option<String>,
}

/// Summary of qualitative evaluation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualitativeEvaluationSummary {
    /// Total number of entries generated
    pub total_entries: usize,
    /// Number of entry categories evaluated
    pub categories_evaluated: usize,
    /// Version of entries configuration used
    pub entries_version: String,
}

impl AIQualitativeOutput {
    /// Create a new AIQualitativeOutput with the given entries
    pub fn new(entries: HashMap<String, Vec<String>>, entries_version: String) -> Self {
        let total_entries: usize = entries.values().map(|v| v.len()).sum();
        let categories_evaluated = entries.len();

        Self {
            entries,
            summary: Some(QualitativeEvaluationSummary {
                total_entries,
                categories_evaluated,
                entries_version: entries_version.clone(),
            }),
            entries_version: Some(entries_version),
        }
    }

    /// Get entries by key
    pub fn get_entries(&self, key: &str) -> Option<&Vec<String>> {
        self.entries.get(key)
    }

    /// Get insights as markdown strings
    pub fn insights(&self) -> Vec<String> {
        self.get_entries("insights").cloned().unwrap_or_default()
    }

    /// Get good patterns as markdown strings
    pub fn good_patterns(&self) -> Vec<String> {
        self.get_entries("good_patterns")
            .cloned()
            .unwrap_or_default()
    }

    /// Get improvement areas as markdown strings
    pub fn improvement_areas(&self) -> Vec<String> {
        self.get_entries("improvement_areas")
            .cloned()
            .unwrap_or_default()
    }

    /// Get recommendations as markdown strings
    pub fn recommendations(&self) -> Vec<String> {
        self.get_entries("recommendations")
            .cloned()
            .unwrap_or_default()
    }

    /// Get learning observations as markdown strings
    pub fn learning_observations(&self) -> Vec<String> {
        self.get_entries("learning_observations")
            .cloned()
            .unwrap_or_default()
    }
}

// =============================================================================
// AI Quantitative Output Models (LLM-as-a-judge rubric evaluation)
// =============================================================================

/// AI-generated quantitative output from rubric-based LLM-as-a-judge evaluation
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AIQuantitativeOutput {
    /// Rubric-based evaluation scores (LLM-as-a-judge)
    #[serde(default)]
    pub rubric_scores: Vec<RubricScore>,
    /// Summary of rubric evaluation
    #[serde(default)]
    pub rubric_summary: Option<RubricEvaluationSummary>,
}
