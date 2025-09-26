use serde::{Deserialize, Serialize};

/// Simple configuration for the hardcoded prompt system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptConfig {
    /// Whether analysis is enabled
    pub enabled: bool,
    /// Custom instruction to add to the default prompt (optional)
    pub custom_instruction: Option<String>,
}

impl PromptConfig {
    pub fn new() -> Self {
        Self {
            enabled: true,
            custom_instruction: None,
        }
    }

    pub fn get_custom_prompt(&self, chat_content: &str) -> String {
        let base_prompt = r#"You are an AI assistant that analyzes chat conversations and provides insights.

Please analyze the following chat content and provide a detailed analysis:

{chat_content}

Focus on:
- Main themes and topics discussed
- Communication patterns
- Key insights or takeaways
- Overall tone and sentiment

Provide a comprehensive analysis in a clear, structured format."#;

        let mut prompt = base_prompt.replace("{chat_content}", chat_content);

        if let Some(custom) = &self.custom_instruction {
            prompt.push_str("\n\nAdditional instructions: ");
            prompt.push_str(custom);
        }

        prompt
    }
}

impl Default for PromptConfig {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prompt_config_creation() {
        let config = PromptConfig::new();
        assert!(config.enabled);
        assert!(config.custom_instruction.is_none());
    }

    #[test]
    fn test_custom_prompt_generation() {
        let mut config = PromptConfig::new();
        config.custom_instruction = Some("Focus specifically on technical aspects.".to_string());

        let prompt = config.get_custom_prompt("Hello world");
        assert!(prompt.contains("Hello world"));
        assert!(prompt.contains("Focus specifically on technical aspects."));
    }
}
