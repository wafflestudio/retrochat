use clap::ValueEnum;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Provider {
    /// All providers (CLI-only, used to import from all providers)
    All,
    ClaudeCode,
    GeminiCLI,
    Codex,
    Other(String),
}

// Implement ValueEnum manually because we need to exclude Other variant
impl ValueEnum for Provider {
    fn value_variants<'a>() -> &'a [Self] {
        &[Self::All, Self::ClaudeCode, Self::GeminiCLI, Self::Codex]
    }

    fn to_possible_value(&self) -> Option<clap::builder::PossibleValue> {
        match self {
            Self::All => Some(clap::builder::PossibleValue::new("all")),
            Self::ClaudeCode => Some(clap::builder::PossibleValue::new("claude")),
            Self::GeminiCLI => Some(clap::builder::PossibleValue::new("gemini")),
            Self::Codex => Some(clap::builder::PossibleValue::new("codex")),
            Self::Other(_) => None,
        }
    }
}

impl std::fmt::Display for Provider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Provider::All => write!(f, "All"),
            Provider::ClaudeCode => write!(f, "Claude Code"),
            Provider::GeminiCLI => write!(f, "Gemini CLI"),
            Provider::Codex => write!(f, "Codex"),
            Provider::Other(name) => write!(f, "{name}"),
        }
    }
}

impl std::str::FromStr for Provider {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "All" | "all" => Ok(Provider::All),
            "Claude Code" | "claude" => Ok(Provider::ClaudeCode),
            "Gemini CLI" | "gemini" => Ok(Provider::GeminiCLI),
            "Codex" | "codex" => Ok(Provider::Codex),
            _ => Ok(Provider::Other(s.to_string())),
        }
    }
}

impl Provider {
    /// Get all concrete provider variants (excluding All and Other)
    pub fn all_concrete() -> Vec<Self> {
        vec![Self::ClaudeCode, Self::GeminiCLI, Self::Codex]
    }

    /// Check if this is a concrete provider (not All or Other)
    pub fn is_concrete(&self) -> bool {
        !matches!(self, Self::All | Self::Other(_))
    }

    /// Expand a list of providers, replacing All with all concrete providers
    pub fn expand_all(providers: Vec<Self>) -> Vec<Self> {
        if providers.contains(&Self::All) {
            Self::all_concrete()
        } else {
            providers
        }
    }
}
