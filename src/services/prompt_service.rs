use anyhow::Result;

const DEFAULT_PROMPT: &str = r#"
You are an AI assistant that analyzes chat conversations and provides insights.

Please analyze the following chat content and provide a detailed analysis:

{chat_content}

Focus on:
- Main themes and topics discussed
- Communication patterns
- Key insights or takeaways
- Overall tone and sentiment

Provide a comprehensive analysis in a clear, structured format.
"#;

#[derive(Debug, Clone)]
pub struct PromptService;

impl Default for PromptService {
    fn default() -> Self {
        Self::new()
    }
}

impl PromptService {
    pub fn new() -> Self {
        Self
    }

    pub fn render_prompt(&self, chat_content: &str) -> Result<String> {
        let rendered = DEFAULT_PROMPT.replace("{chat_content}", chat_content);
        Ok(rendered)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_prompt() {
        let service = PromptService::new();
        let chat_content = "Hello, how are you? I'm doing well, thanks for asking!";

        let rendered = service.render_prompt(chat_content).unwrap();
        assert!(rendered.contains(chat_content));
        assert!(rendered.contains("analyze"));
        assert!(rendered.contains("insights"));
    }
}
