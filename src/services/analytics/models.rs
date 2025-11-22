use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
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
    /// JSON schema describing the structure of each item
    pub item_schema: HashMap<String, String>,
    /// Minimum number of items to generate
    #[serde(default = "default_min_items")]
    pub min_items: u32,
    /// Maximum number of items to generate
    #[serde(default = "default_max_items")]
    pub max_items: u32,
}

fn default_min_items() -> u32 {
    1
}

fn default_max_items() -> u32 {
    3
}

impl QualitativeEntry {
    /// Format entry for inclusion in LLM prompts
    pub fn format_for_prompt(&self) -> String {
        let schema_lines: Vec<String> = self
            .item_schema
            .iter()
            .map(|(k, v)| format!("      \"{}\": \"{}\"", k, v))
            .collect();

        format!(
            r#"**{}**: {} ({}-{} items)
  [
    {{
{}
    }}
  ]"#,
            self.title,
            self.description,
            self.min_items,
            self.max_items,
            schema_lines.join(",\n")
        )
    }

    /// Format JSON schema for the entry
    pub fn format_json_schema(&self) -> String {
        let schema_lines: Vec<String> = self
            .item_schema
            .iter()
            .map(|(k, v)| format!("        \"{}\": {}", k, v))
            .collect();

        format!(
            r#"  "{}": [
    {{
{}
    }}
  ]"#,
            self.key,
            schema_lines.join(",\n")
        )
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

    /// Format all entries for inclusion in LLM prompt
    pub fn format_for_prompt(&self) -> String {
        self.entries
            .iter()
            .enumerate()
            .map(|(i, e)| format!("{}. {}", i + 1, e.format_for_prompt()))
            .collect::<Vec<_>>()
            .join("\n\n")
    }

    /// Format expected JSON schema for LLM output
    pub fn format_json_schema(&self) -> String {
        let schemas: Vec<String> = self
            .entries
            .iter()
            .map(|e| e.format_json_schema())
            .collect();
        format!("{{\n{}\n}}", schemas.join(",\n"))
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
    /// Tool uses embedded in this message (if any)
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub tool_uses: Vec<EmbeddedToolUse>,
}

/// Represents a tool use embedded within a message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddedToolUse {
    /// Tool name (e.g., "Read", "Write", "Edit", "Bash")
    pub tool_name: String,
    /// Tool input/request (truncated if too long)
    pub input: String,
    /// Tool result/response (truncated if too long)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<String>,
    /// Whether the tool execution was successful
    #[serde(skip_serializing_if = "Option::is_none")]
    pub success: Option<bool>,
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
// AI Qualitative Output Models (configurable LLM-based qualitative analysis)
// =============================================================================

/// AI-generated qualitative output from configurable entry-based analysis
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AIQualitativeOutput {
    /// Dynamic entries based on qualitative_entries.json configuration
    /// Key is the entry key (e.g., "insights"), value is an array of items
    #[serde(default)]
    pub entries: HashMap<String, Vec<JsonValue>>,
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
    pub fn new(entries: HashMap<String, Vec<JsonValue>>, entries_version: String) -> Self {
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
    pub fn get_entries(&self, key: &str) -> Option<&Vec<JsonValue>> {
        self.entries.get(key)
    }

    /// Get insights (convenience method for backward compatibility)
    pub fn insights(&self) -> Vec<Insight> {
        self.get_entries("insights")
            .map(|items| {
                items
                    .iter()
                    .filter_map(|v| serde_json::from_value(v.clone()).ok())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get good patterns (convenience method for backward compatibility)
    pub fn good_patterns(&self) -> Vec<GoodPattern> {
        self.get_entries("good_patterns")
            .map(|items| {
                items
                    .iter()
                    .filter_map(|v| serde_json::from_value(v.clone()).ok())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get improvement areas (convenience method for backward compatibility)
    pub fn improvement_areas(&self) -> Vec<ImprovementArea> {
        self.get_entries("improvement_areas")
            .map(|items| {
                items
                    .iter()
                    .filter_map(|v| serde_json::from_value(v.clone()).ok())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get recommendations (convenience method for backward compatibility)
    pub fn recommendations(&self) -> Vec<Recommendation> {
        self.get_entries("recommendations")
            .map(|items| {
                items
                    .iter()
                    .filter_map(|v| serde_json::from_value(v.clone()).ok())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get learning observations (convenience method for backward compatibility)
    pub fn learning_observations(&self) -> Vec<LearningObservation> {
        self.get_entries("learning_observations")
            .map(|items| {
                items
                    .iter()
                    .filter_map(|v| serde_json::from_value(v.clone()).ok())
                    .collect()
            })
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

// =============================================================================
// Qualitative Entry Item Types (for backward compatibility and typed access)
// =============================================================================

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
