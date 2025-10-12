use chrono::{DateTime, Utc, Weekday};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::llm_provider::LlmProvider;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AnalysisType {
    Daily,
    Weekly,
    Monthly,
    Provider,
    Project,
    Custom,
}

impl std::fmt::Display for AnalysisType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AnalysisType::Daily => write!(f, "Daily"),
            AnalysisType::Weekly => write!(f, "Weekly"),
            AnalysisType::Monthly => write!(f, "Monthly"),
            AnalysisType::Provider => write!(f, "Provider"),
            AnalysisType::Project => write!(f, "Project"),
            AnalysisType::Custom => write!(f, "Custom"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PurposeCategory {
    pub name: String,
    pub percentage: f64,
    pub session_count: u32,
    pub examples: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityScore {
    pub metric: String,
    pub score: f64,
    pub description: String,
    pub suggestions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Recommendation {
    pub title: String,
    pub description: String,
    pub priority: RecommendationPriority,
    pub action_items: Vec<String>,
    pub estimated_impact: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RecommendationPriority {
    High,
    Medium,
    Low,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageAnalysis {
    pub id: Uuid,
    pub analysis_type: AnalysisType,
    pub time_period_start: DateTime<Utc>,
    pub time_period_end: DateTime<Utc>,
    pub provider_filter: Option<LlmProvider>,
    pub project_filter: Option<String>,
    pub total_sessions: u32,
    pub total_messages: u32,
    pub total_tokens: u64,
    pub average_session_length: f64,
    pub most_active_day: Option<Weekday>,
    pub purpose_categories: Vec<PurposeCategory>,
    pub quality_scores: Vec<QualityScore>,
    pub recommendations: Vec<Recommendation>,
    pub generated_at: DateTime<Utc>,
}

impl UsageAnalysis {
    pub fn new(
        analysis_type: AnalysisType,
        time_period_start: DateTime<Utc>,
        time_period_end: DateTime<Utc>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            analysis_type,
            time_period_start,
            time_period_end,
            provider_filter: None,
            project_filter: None,
            total_sessions: 0,
            total_messages: 0,
            total_tokens: 0,
            average_session_length: 0.0,
            most_active_day: None,
            purpose_categories: Vec::new(),
            quality_scores: Vec::new(),
            recommendations: Vec::new(),
            generated_at: Utc::now(),
        }
    }

    pub fn with_provider_filter(mut self, provider: LlmProvider) -> Self {
        self.provider_filter = Some(provider);
        self
    }

    pub fn with_project_filter(mut self, project: String) -> Self {
        self.project_filter = Some(project);
        self
    }

    pub fn set_metrics(&mut self, total_sessions: u32, total_messages: u32, total_tokens: u64) {
        self.total_sessions = total_sessions;
        self.total_messages = total_messages;
        self.total_tokens = total_tokens;
        self.average_session_length = if total_sessions > 0 {
            total_messages as f64 / total_sessions as f64
        } else {
            0.0
        };
    }

    pub fn add_purpose_category(&mut self, category: PurposeCategory) {
        self.purpose_categories.push(category);
    }

    pub fn add_quality_score(&mut self, score: QualityScore) {
        self.quality_scores.push(score);
    }

    pub fn add_recommendation(&mut self, recommendation: Recommendation) {
        self.recommendations.push(recommendation);
    }

    pub fn set_most_active_day(&mut self, day: Weekday) {
        self.most_active_day = Some(day);
    }

    pub fn duration(&self) -> chrono::Duration {
        self.time_period_end - self.time_period_start
    }

    pub fn duration_days(&self) -> i64 {
        self.duration().num_days()
    }

    pub fn average_tokens_per_session(&self) -> f64 {
        if self.total_sessions > 0 {
            self.total_tokens as f64 / self.total_sessions as f64
        } else {
            0.0
        }
    }

    pub fn average_tokens_per_message(&self) -> f64 {
        if self.total_messages > 0 {
            self.total_tokens as f64 / self.total_messages as f64
        } else {
            0.0
        }
    }

    pub fn sessions_per_day(&self) -> f64 {
        let days = self.duration_days();
        if days > 0 {
            self.total_sessions as f64 / days as f64
        } else {
            0.0
        }
    }

    pub fn is_valid(&self) -> bool {
        self.time_period_end > self.time_period_start
    }

    pub fn get_high_priority_recommendations(&self) -> Vec<&Recommendation> {
        self.recommendations
            .iter()
            .filter(|r| r.priority == RecommendationPriority::High)
            .collect()
    }

    pub fn get_primary_purpose(&self) -> Option<&PurposeCategory> {
        self.purpose_categories
            .iter()
            .max_by(|a, b| a.percentage.partial_cmp(&b.percentage).unwrap())
    }

    pub fn get_quality_score(&self, metric: &str) -> Option<&QualityScore> {
        self.quality_scores.iter().find(|s| s.metric == metric)
    }
}

impl PurposeCategory {
    pub fn new(name: String, percentage: f64, session_count: u32) -> Self {
        Self {
            name,
            percentage,
            session_count,
            examples: Vec::new(),
        }
    }

    pub fn with_examples(mut self, examples: Vec<String>) -> Self {
        self.examples = examples;
        self
    }
}

impl QualityScore {
    pub fn new(metric: String, score: f64, description: String) -> Self {
        Self {
            metric,
            score,
            description,
            suggestions: Vec::new(),
        }
    }

    pub fn with_suggestions(mut self, suggestions: Vec<String>) -> Self {
        self.suggestions = suggestions;
        self
    }
}

impl Recommendation {
    pub fn new(title: String, description: String, priority: RecommendationPriority) -> Self {
        Self {
            title,
            description,
            priority,
            action_items: Vec::new(),
            estimated_impact: String::new(),
        }
    }

    pub fn with_action_items(mut self, action_items: Vec<String>) -> Self {
        self.action_items = action_items;
        self
    }

    pub fn with_estimated_impact(mut self, impact: String) -> Self {
        self.estimated_impact = impact;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_usage_analysis() {
        let start = Utc::now() - chrono::Duration::days(7);
        let end = Utc::now();

        let analysis = UsageAnalysis::new(AnalysisType::Weekly, start, end);

        assert_eq!(analysis.analysis_type, AnalysisType::Weekly);
        assert_eq!(analysis.time_period_start, start);
        assert_eq!(analysis.time_period_end, end);
        assert!(analysis.is_valid());
        assert_eq!(analysis.total_sessions, 0);
    }

    #[test]
    fn test_invalid_time_period() {
        let start = Utc::now();
        let end = start - chrono::Duration::hours(1);

        let analysis = UsageAnalysis::new(AnalysisType::Daily, start, end);
        assert!(!analysis.is_valid());
    }

    #[test]
    fn test_set_metrics() {
        let mut analysis = UsageAnalysis::new(
            AnalysisType::Daily,
            Utc::now() - chrono::Duration::days(1),
            Utc::now(),
        );

        analysis.set_metrics(10, 50, 1000);

        assert_eq!(analysis.total_sessions, 10);
        assert_eq!(analysis.total_messages, 50);
        assert_eq!(analysis.total_tokens, 1000);
        assert_eq!(analysis.average_session_length, 5.0);
        assert_eq!(analysis.average_tokens_per_session(), 100.0);
        assert_eq!(analysis.average_tokens_per_message(), 20.0);
    }

    #[test]
    fn test_duration_calculations() {
        let start = Utc::now() - chrono::Duration::days(7);
        let end = Utc::now();

        let mut analysis = UsageAnalysis::new(AnalysisType::Weekly, start, end);
        analysis.set_metrics(14, 70, 1400);

        assert_eq!(analysis.duration_days(), 7);
        assert_eq!(analysis.sessions_per_day(), 2.0);
    }

    #[test]
    fn test_purpose_categories() {
        let mut analysis = UsageAnalysis::new(
            AnalysisType::Daily,
            Utc::now() - chrono::Duration::days(1),
            Utc::now(),
        );

        let category1 = PurposeCategory::new("Coding".to_string(), 60.0, 6);
        let category2 = PurposeCategory::new("Research".to_string(), 40.0, 4);

        analysis.add_purpose_category(category1);
        analysis.add_purpose_category(category2);

        let primary = analysis.get_primary_purpose().unwrap();
        assert_eq!(primary.name, "Coding");
        assert_eq!(primary.percentage, 60.0);
    }

    #[test]
    fn test_recommendations() {
        let mut analysis = UsageAnalysis::new(
            AnalysisType::Daily,
            Utc::now() - chrono::Duration::days(1),
            Utc::now(),
        );

        let high_rec = Recommendation::new(
            "Optimize queries".to_string(),
            "Reduce token usage".to_string(),
            RecommendationPriority::High,
        );

        let low_rec = Recommendation::new(
            "Explore features".to_string(),
            "Try new capabilities".to_string(),
            RecommendationPriority::Low,
        );

        analysis.add_recommendation(high_rec);
        analysis.add_recommendation(low_rec);

        let high_priority = analysis.get_high_priority_recommendations();
        assert_eq!(high_priority.len(), 1);
        assert_eq!(high_priority[0].title, "Optimize queries");
    }

    #[test]
    fn test_quality_scores() {
        let mut analysis = UsageAnalysis::new(
            AnalysisType::Daily,
            Utc::now() - chrono::Duration::days(1),
            Utc::now(),
        );

        let score = QualityScore::new(
            "efficiency".to_string(),
            85.5,
            "Good efficiency".to_string(),
        );

        analysis.add_quality_score(score);

        let retrieved = analysis.get_quality_score("efficiency").unwrap();
        assert_eq!(retrieved.score, 85.5);
    }
}
