use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::path::Path;

// =============================================================================
// Qualitative Category Models (for configurable qualitative output)
// =============================================================================

/// Schema definition for metadata fields in a qualitative category
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetadataSchema {
    /// Key name for this metadata field
    pub key: String,
    /// Display name for UI
    pub display_name: String,
    /// Type of value: "string", "number", "array"
    pub value_type: String,
    /// Description of what this field represents
    pub description: String,
}

/// A qualitative category definition loaded from JSON
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualitativeCategory {
    /// Unique identifier for the category (e.g., "insight", "good_pattern")
    pub id: String,
    /// Display name (e.g., "Key Insights")
    pub name: String,
    /// What this category captures
    pub description: String,
    /// Icon identifier for UI (e.g., "lightbulb", "check")
    pub icon: String,
    /// Schema for metadata fields
    pub metadata_schema: Vec<MetadataSchema>,
}

/// Container for qualitative categories with version
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualitativeCategoryList {
    /// Schema version
    #[serde(default = "default_category_version")]
    pub version: String,
    /// List of categories
    pub categories: Vec<QualitativeCategory>,
}

fn default_category_version() -> String {
    "1.0".to_string()
}

impl QualitativeCategoryList {
    /// Load categories from a JSON file
    pub fn from_json_file(path: &Path) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let category_list: QualitativeCategoryList = serde_json::from_str(&content)?;
        Ok(category_list)
    }

    /// Load categories from embedded JSON string
    pub fn from_json_str(json: &str) -> anyhow::Result<Self> {
        let category_list: QualitativeCategoryList = serde_json::from_str(json)?;
        Ok(category_list)
    }

    /// Get default categories (embedded in binary)
    pub fn default_categories() -> Self {
        let json = include_str!("../../../resources/qualitative_categories.json");
        Self::from_json_str(json).expect("Default qualitative categories should be valid JSON")
    }

    /// Get a category by ID
    pub fn get_category(&self, id: &str) -> Option<&QualitativeCategory> {
        self.categories.iter().find(|c| c.id == id)
    }
}

/// A generic qualitative item that can represent any type of insight/pattern/recommendation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualitativeItem {
    /// Category ID (references QualitativeCategory.id)
    pub category_id: String,
    /// Primary title/name
    pub title: String,
    /// Detailed description
    pub description: String,
    /// Flexible metadata fields (category-specific)
    #[serde(default)]
    pub metadata: HashMap<String, Value>,
}

impl QualitativeItem {
    /// Create a new item with the given category, title, and description
    pub fn new(category_id: &str, title: &str, description: &str) -> Self {
        Self {
            category_id: category_id.to_string(),
            title: title.to_string(),
            description: description.to_string(),
            metadata: HashMap::new(),
        }
    }

    /// Add a string metadata field
    pub fn with_string(mut self, key: &str, value: &str) -> Self {
        self.metadata
            .insert(key.to_string(), Value::String(value.to_string()));
        self
    }

    /// Add a number metadata field
    pub fn with_number(mut self, key: &str, value: f64) -> Self {
        self.metadata.insert(
            key.to_string(),
            Value::Number(
                serde_json::Number::from_f64(value).unwrap_or(serde_json::Number::from(0)),
            ),
        );
        self
    }

    /// Add an array metadata field
    pub fn with_array(mut self, key: &str, values: Vec<String>) -> Self {
        self.metadata.insert(
            key.to_string(),
            Value::Array(values.into_iter().map(Value::String).collect()),
        );
        self
    }

    /// Get a string metadata value
    pub fn get_string(&self, key: &str) -> Option<&str> {
        self.metadata.get(key).and_then(|v| v.as_str())
    }

    /// Get a number metadata value
    pub fn get_number(&self, key: &str) -> Option<f64> {
        self.metadata.get(key).and_then(|v| v.as_f64())
    }

    /// Get an array metadata value
    pub fn get_array(&self, key: &str) -> Option<Vec<&str>> {
        self.metadata.get(key).and_then(|v| {
            v.as_array()
                .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect())
        })
    }
}

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualitativeInput {
    pub file_contexts: Vec<FileContext>,
    pub chat_context: ChatContext,
    pub project_context: ProjectContext,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileContext {
    pub file_path: String,
    pub file_type: String,
    pub modification_type: String,
    pub content_snippet: String,
    pub complexity_indicators: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatContext {
    pub conversation_flow: String,
    pub problem_solving_patterns: Vec<String>,
    pub ai_interaction_quality: f64,
    pub key_topics: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectContext {
    pub project_type: String,
    pub technology_stack: Vec<String>,
    pub project_complexity: f64,
    pub development_stage: String,
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
    /// All qualitative items grouped by category_id
    pub items: Vec<QualitativeItem>,
    /// Rubric-based evaluation scores (LLM-as-a-judge)
    #[serde(default)]
    pub rubric_scores: Vec<RubricScore>,
    /// Summary of rubric evaluation
    #[serde(default)]
    pub rubric_summary: Option<RubricEvaluationSummary>,
}

impl QualitativeOutput {
    /// Create an empty QualitativeOutput
    pub fn empty() -> Self {
        Self {
            items: Vec::new(),
            rubric_scores: Vec::new(),
            rubric_summary: None,
        }
    }

    /// Get items filtered by category ID
    pub fn items_by_category(&self, category_id: &str) -> Vec<&QualitativeItem> {
        self.items
            .iter()
            .filter(|item| item.category_id == category_id)
            .collect()
    }

    /// Add an item to the output
    pub fn add_item(&mut self, item: QualitativeItem) {
        self.items.push(item);
    }

    /// Get all unique category IDs present in items
    pub fn category_ids(&self) -> Vec<String> {
        let mut ids: Vec<String> = self
            .items
            .iter()
            .map(|item| item.category_id.clone())
            .collect();
        ids.sort();
        ids.dedup();
        ids
    }
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
