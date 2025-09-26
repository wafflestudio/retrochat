use serde::{Deserialize, Serialize};

/// Tracks analysis execution details and costs
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AnalysisMetadata {
    /// LLM service used ("gemini-2.5-flash-lite")
    pub llm_service: String,
    /// Input tokens consumed
    pub prompt_tokens: u32,
    /// Output tokens generated
    pub completion_tokens: u32,
    /// Total tokens used
    pub total_tokens: u32,
    /// Estimated API cost in USD
    pub estimated_cost: f64,
    /// Analysis duration in milliseconds
    pub execution_time_ms: u64,
    /// Raw API metadata JSON for debugging
    pub api_response_metadata: Option<String>,
}

impl AnalysisMetadata {
    /// Create new metadata with basic information
    pub fn new(
        llm_service: String,
        prompt_tokens: u32,
        completion_tokens: u32,
        execution_time_ms: u64,
    ) -> Self {
        let total_tokens = prompt_tokens + completion_tokens;
        let estimated_cost = Self::calculate_cost(&llm_service, prompt_tokens, completion_tokens);

        Self {
            llm_service,
            prompt_tokens,
            completion_tokens,
            total_tokens,
            estimated_cost,
            execution_time_ms,
            api_response_metadata: None,
        }
    }

    /// Create metadata with API response details
    pub fn with_api_metadata(
        llm_service: String,
        prompt_tokens: u32,
        completion_tokens: u32,
        execution_time_ms: u64,
        api_response_metadata: String,
    ) -> Self {
        let mut metadata = Self::new(
            llm_service,
            prompt_tokens,
            completion_tokens,
            execution_time_ms,
        );
        metadata.api_response_metadata = Some(api_response_metadata);
        metadata
    }

    /// Calculate estimated cost based on LLM service and token usage
    fn calculate_cost(llm_service: &str, prompt_tokens: u32, completion_tokens: u32) -> f64 {
        match llm_service {
            "gemini-2.5-flash-lite" => {
                // Gemini 2.5 Flash Lite pricing (as of 2025)
                // Input: $0.10 per 1M tokens
                // Output: $0.40 per 1M tokens
                let input_cost = (prompt_tokens as f64 / 1_000_000.0) * 0.10;
                let output_cost = (completion_tokens as f64 / 1_000_000.0) * 0.40;
                input_cost + output_cost
            }
            "gemini-2.5-flash" => {
                // Gemini 2.5 Flash pricing
                // Input: $0.20 per 1M tokens
                // Output: $0.60 per 1M tokens
                let input_cost = (prompt_tokens as f64 / 1_000_000.0) * 0.20;
                let output_cost = (completion_tokens as f64 / 1_000_000.0) * 0.60;
                input_cost + output_cost
            }
            "gemini-2.5-pro" => {
                // Gemini 2.5 Pro pricing
                // Input: $2.50 per 1M tokens
                // Output: $10.00 per 1M tokens
                let input_cost = (prompt_tokens as f64 / 1_000_000.0) * 2.50;
                let output_cost = (completion_tokens as f64 / 1_000_000.0) * 10.00;
                input_cost + output_cost
            }
            _ => {
                // Unknown service, use Flash Lite pricing as default
                let input_cost = (prompt_tokens as f64 / 1_000_000.0) * 0.10;
                let output_cost = (completion_tokens as f64 / 1_000_000.0) * 0.40;
                input_cost + output_cost
            }
        }
    }

    /// Get cost per token for this service
    pub fn get_cost_per_token(&self) -> (f64, f64) {
        match self.llm_service.as_str() {
            "gemini-2.5-flash-lite" => (0.10 / 1_000_000.0, 0.40 / 1_000_000.0),
            "gemini-2.5-flash" => (0.20 / 1_000_000.0, 0.60 / 1_000_000.0),
            "gemini-2.5-pro" => (2.50 / 1_000_000.0, 10.00 / 1_000_000.0),
            _ => (0.10 / 1_000_000.0, 0.40 / 1_000_000.0), // Default to Flash Lite
        }
    }

    /// Get execution time in seconds
    pub fn get_execution_time_seconds(&self) -> f64 {
        self.execution_time_ms as f64 / 1000.0
    }

    /// Get tokens per second processing rate
    pub fn get_tokens_per_second(&self) -> f64 {
        if self.execution_time_ms == 0 {
            return 0.0;
        }
        (self.total_tokens as f64) / self.get_execution_time_seconds()
    }

    /// Get cost efficiency (tokens per cent)
    pub fn get_cost_efficiency(&self) -> f64 {
        if self.estimated_cost == 0.0 {
            return 0.0;
        }
        (self.total_tokens as f64) / (self.estimated_cost * 100.0)
    }

    /// Format cost as human-readable string
    pub fn format_cost(&self) -> String {
        if self.estimated_cost < 0.001 {
            format!("${:.4}", self.estimated_cost)
        } else if self.estimated_cost < 0.01 {
            format!("${:.3}", self.estimated_cost)
        } else {
            format!("${:.2}", self.estimated_cost)
        }
    }

    /// Format execution time as human-readable string
    pub fn format_execution_time(&self) -> String {
        let seconds = self.get_execution_time_seconds();
        if seconds < 1.0 {
            format!("{}ms", self.execution_time_ms)
        } else if seconds < 60.0 {
            format!("{seconds:.1}s")
        } else {
            let minutes = seconds / 60.0;
            format!("{minutes:.1}m")
        }
    }

    /// Get a performance summary
    pub fn get_performance_summary(&self) -> String {
        format!(
            "{} tokens in {} (${}) - {:.1} tokens/sec",
            self.total_tokens,
            self.format_execution_time(),
            self.format_cost(),
            self.get_tokens_per_second()
        )
    }

    /// Check if this is an expensive analysis (over threshold)
    pub fn is_expensive(&self, threshold_usd: f64) -> bool {
        self.estimated_cost > threshold_usd
    }

    /// Check if this is a slow analysis (over threshold)
    pub fn is_slow(&self, threshold_seconds: f64) -> bool {
        self.get_execution_time_seconds() > threshold_seconds
    }

    /// Estimate future cost for similar analysis
    pub fn estimate_cost_for_tokens(&self, prompt_tokens: u32, completion_tokens: u32) -> f64 {
        Self::calculate_cost(&self.llm_service, prompt_tokens, completion_tokens)
    }

    /// Get token distribution ratio (completion/prompt)
    pub fn get_token_ratio(&self) -> f64 {
        if self.prompt_tokens == 0 {
            return 0.0;
        }
        self.completion_tokens as f64 / self.prompt_tokens as f64
    }

    /// Check if token usage is balanced (not too much input vs output)
    pub fn is_token_usage_balanced(&self) -> bool {
        let ratio = self.get_token_ratio();
        // Balanced if output is 10%-500% of input
        (0.1..=5.0).contains(&ratio)
    }

    /// Validate metadata consistency
    pub fn validate(&self) -> Result<(), String> {
        if self.total_tokens != self.prompt_tokens + self.completion_tokens {
            return Err(
                "Total tokens does not match sum of prompt and completion tokens".to_string(),
            );
        }

        if self.estimated_cost < 0.0 {
            return Err("Estimated cost cannot be negative".to_string());
        }

        if self.llm_service.is_empty() {
            return Err("LLM service cannot be empty".to_string());
        }

        // Sanity check for extreme values
        if self.total_tokens > 10_000_000 {
            return Err("Token count seems unreasonably high".to_string());
        }

        if self.execution_time_ms > 10 * 60 * 1000 {
            // 10 minutes
            return Err("Execution time seems unreasonably long".to_string());
        }

        Ok(())
    }
}

impl Default for AnalysisMetadata {
    fn default() -> Self {
        Self {
            llm_service: "gemini-2.5-flash-lite".to_string(),
            prompt_tokens: 0,
            completion_tokens: 0,
            total_tokens: 0,
            estimated_cost: 0.0,
            execution_time_ms: 0,
            api_response_metadata: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metadata_creation() {
        let metadata = AnalysisMetadata::new("gemini-2.5-flash-lite".to_string(), 100, 50, 1500);

        assert_eq!(metadata.llm_service, "gemini-2.5-flash-lite");
        assert_eq!(metadata.prompt_tokens, 100);
        assert_eq!(metadata.completion_tokens, 50);
        assert_eq!(metadata.total_tokens, 150);
        assert_eq!(metadata.execution_time_ms, 1500);
        assert!(metadata.estimated_cost > 0.0);
    }

    #[test]
    fn test_cost_calculation() {
        // Test Flash Lite pricing
        let cost = AnalysisMetadata::calculate_cost("gemini-2.5-flash-lite", 1_000_000, 1_000_000);
        assert_eq!(cost, 0.50); // $0.10 + $0.40

        // Test Flash pricing
        let cost = AnalysisMetadata::calculate_cost("gemini-2.5-flash", 1_000_000, 1_000_000);
        assert_eq!(cost, 0.80); // $0.20 + $0.60

        // Test Pro pricing
        let cost = AnalysisMetadata::calculate_cost("gemini-2.5-pro", 1_000_000, 1_000_000);
        assert_eq!(cost, 12.50); // $2.50 + $10.00

        // Test unknown service (defaults to Flash Lite)
        let cost = AnalysisMetadata::calculate_cost("unknown-service", 1_000_000, 1_000_000);
        assert_eq!(cost, 0.50);
    }

    #[test]
    fn test_performance_metrics() {
        let metadata = AnalysisMetadata::new(
            "gemini-2.5-flash-lite".to_string(),
            1000,
            500,
            2000, // 2 seconds
        );

        assert_eq!(metadata.get_execution_time_seconds(), 2.0);
        assert_eq!(metadata.get_tokens_per_second(), 750.0); // 1500 tokens / 2 seconds
        assert_eq!(metadata.get_token_ratio(), 0.5); // 500/1000

        let efficiency = metadata.get_cost_efficiency();
        assert!(efficiency > 0.0);
    }

    #[test]
    fn test_formatting() {
        let metadata = AnalysisMetadata::new("gemini-2.5-flash-lite".to_string(), 100, 50, 1500);

        assert!(metadata.format_cost().starts_with('$'));
        assert!(metadata.format_execution_time().contains("1.5s"));

        let summary = metadata.get_performance_summary();
        assert!(summary.contains("150 tokens"));
        assert!(summary.contains("1.5s"));
        assert!(summary.contains("tokens/sec"));
    }

    #[test]
    fn test_thresholds() {
        let expensive_metadata =
            AnalysisMetadata::new("gemini-2.5-pro".to_string(), 1_000_000, 1_000_000, 1000);

        assert!(expensive_metadata.is_expensive(1.0));
        assert!(!expensive_metadata.is_expensive(20.0));

        let slow_metadata = AnalysisMetadata::new(
            "gemini-2.5-flash-lite".to_string(),
            100,
            50,
            10_000, // 10 seconds
        );

        assert!(slow_metadata.is_slow(5.0));
        assert!(!slow_metadata.is_slow(15.0));
    }

    #[test]
    fn test_token_balance() {
        // Balanced usage
        let balanced = AnalysisMetadata::new(
            "gemini-2.5-flash-lite".to_string(),
            100,
            50, // 0.5 ratio
            1000,
        );
        assert!(balanced.is_token_usage_balanced());

        // Too much input, too little output
        let imbalanced_low = AnalysisMetadata::new(
            "gemini-2.5-flash-lite".to_string(),
            100,
            5, // 0.05 ratio
            1000,
        );
        assert!(!imbalanced_low.is_token_usage_balanced());

        // Too much output relative to input
        let imbalanced_high = AnalysisMetadata::new(
            "gemini-2.5-flash-lite".to_string(),
            100,
            800, // 8.0 ratio
            1000,
        );
        assert!(!imbalanced_high.is_token_usage_balanced());
    }

    #[test]
    fn test_validation() {
        // Valid metadata
        let valid = AnalysisMetadata::new("gemini-2.5-flash-lite".to_string(), 100, 50, 1500);
        assert!(valid.validate().is_ok());

        // Invalid total tokens
        let mut invalid = valid.clone();
        invalid.total_tokens = 100; // Should be 150
        assert!(invalid.validate().is_err());

        // Negative cost
        let mut invalid = valid.clone();
        invalid.estimated_cost = -1.0;
        assert!(invalid.validate().is_err());

        // Empty service name
        let mut invalid = valid.clone();
        invalid.llm_service = String::new();
        assert!(invalid.validate().is_err());

        // Extreme values
        let mut invalid = valid.clone();
        invalid.total_tokens = 20_000_000;
        assert!(invalid.validate().is_err());

        let mut invalid = valid.clone();
        invalid.execution_time_ms = 15 * 60 * 1000; // 15 minutes
        assert!(invalid.validate().is_err());
    }

    #[test]
    fn test_cost_estimation() {
        let metadata = AnalysisMetadata::new("gemini-2.5-flash-lite".to_string(), 100, 50, 1000);

        let estimated = metadata.estimate_cost_for_tokens(200, 100);
        let expected = AnalysisMetadata::calculate_cost("gemini-2.5-flash-lite", 200, 100);
        assert_eq!(estimated, expected);
    }

    #[test]
    fn test_with_api_metadata() {
        let api_response = r#"{"model": "gemini-2.5-flash-lite", "usage": {"prompt_tokens": 100}}"#;
        let metadata = AnalysisMetadata::with_api_metadata(
            "gemini-2.5-flash-lite".to_string(),
            100,
            50,
            1500,
            api_response.to_string(),
        );

        assert_eq!(
            metadata.api_response_metadata,
            Some(api_response.to_string())
        );
        assert_eq!(metadata.total_tokens, 150);
    }
}
