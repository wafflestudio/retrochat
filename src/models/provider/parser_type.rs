use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ParserType {
    ClaudeCodeJsonl,
    GeminiJson,
    CodexJson,
    Generic,
}

impl std::fmt::Display for ParserType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParserType::ClaudeCodeJsonl => write!(f, "claude-code-jsonl"),
            ParserType::GeminiJson => write!(f, "gemini-json"),
            ParserType::CodexJson => write!(f, "codex-json"),
            ParserType::Generic => write!(f, "generic"),
        }
    }
}
