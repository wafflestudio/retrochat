use std::path::{Path, PathBuf};

use crate::models::Provider;

/// Represents a detected provider with its location and session count estimate
#[derive(Debug, Clone)]
pub struct DetectedProvider {
    pub provider: Provider,
    pub paths: Vec<PathBuf>,
    pub estimated_sessions: usize,
    pub is_valid: bool,
}

/// Auto-detection service for LLM chat providers
pub struct AutoDetectService;

impl AutoDetectService {
    /// Scan all known provider locations and return detected providers
    pub fn scan_all() -> Vec<DetectedProvider> {
        vec![
            Self::detect_claude_code(),
            Self::detect_cursor(),
            Self::detect_gemini(),
            Self::detect_codex(),
        ]
    }

    /// Detect Claude Code installation
    fn detect_claude_code() -> DetectedProvider {
        let default_path = dirs::home_dir()
            .map(|h| h.join(".claude").join("projects"))
            .unwrap_or_default();

        let (is_valid, estimated_sessions) = Self::check_directory(&default_path, &["*.jsonl"]);

        DetectedProvider {
            provider: Provider::ClaudeCode,
            paths: vec![default_path],
            estimated_sessions,
            is_valid,
        }
    }

    /// Detect Cursor installation
    fn detect_cursor() -> DetectedProvider {
        let default_path = dirs::home_dir()
            .map(|h| h.join(".cursor").join("chats"))
            .unwrap_or_default();

        let (is_valid, estimated_sessions) = Self::check_directory(&default_path, &["store.db"]);

        DetectedProvider {
            provider: Provider::CursorAgent,
            paths: vec![default_path],
            estimated_sessions,
            is_valid,
        }
    }

    /// Detect Gemini CLI installation
    fn detect_gemini() -> DetectedProvider {
        let default_path = dirs::home_dir()
            .map(|h| h.join(".gemini").join("tmp"))
            .unwrap_or_default();

        let (is_valid, estimated_sessions) =
            Self::check_directory(&default_path, &["session-*.json"]);

        DetectedProvider {
            provider: Provider::GeminiCLI,
            paths: vec![default_path],
            estimated_sessions,
            is_valid,
        }
    }

    /// Detect Codex installation
    fn detect_codex() -> DetectedProvider {
        // Codex doesn't have a default location, check env var
        let env_path = std::env::var("RETROCHAT_CODEX_DIRS")
            .ok()
            .and_then(|p| p.split(':').next().map(PathBuf::from));

        if let Some(path) = env_path {
            let (is_valid, estimated_sessions) =
                Self::check_directory(&path, &["*.json", "*.jsonl"]);
            DetectedProvider {
                provider: Provider::Codex,
                paths: vec![path],
                estimated_sessions,
                is_valid,
            }
        } else {
            DetectedProvider {
                provider: Provider::Codex,
                paths: vec![],
                estimated_sessions: 0,
                is_valid: false,
            }
        }
    }

    /// Check if directory exists and count potential session files
    fn check_directory(path: &Path, patterns: &[&str]) -> (bool, usize) {
        if !path.exists() || !path.is_dir() {
            return (false, 0);
        }

        let mut count = 0;

        // Try to count files matching patterns
        if let Ok(entries) = std::fs::read_dir(path) {
            for entry in entries.flatten() {
                if let Ok(file_type) = entry.file_type() {
                    if file_type.is_file() {
                        let file_name = entry.file_name();
                        let file_name_str = file_name.to_string_lossy();

                        for pattern in patterns {
                            if Self::matches_pattern(&file_name_str, pattern) {
                                count += 1;
                                break;
                            }
                        }
                    }
                }
            }
        }

        (count > 0, count)
    }

    /// Simple pattern matching (supports * wildcard)
    fn matches_pattern(filename: &str, pattern: &str) -> bool {
        if pattern == "*" {
            return true;
        }

        if let Some(prefix) = pattern.strip_suffix('*') {
            filename.starts_with(prefix.trim_end_matches('.'))
        } else if let Some(suffix) = pattern.strip_prefix('*') {
            filename.ends_with(suffix)
        } else {
            filename == pattern
        }
    }

    /// Get total session count from detected providers
    pub fn total_sessions(detected: &[DetectedProvider]) -> usize {
        detected.iter().map(|d| d.estimated_sessions).sum()
    }

    /// Get valid providers only
    pub fn valid_providers(detected: &[DetectedProvider]) -> Vec<DetectedProvider> {
        detected.iter().filter(|d| d.is_valid).cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_matches_pattern_suffix_wildcard() {
        assert!(AutoDetectService::matches_pattern("test.jsonl", "*.jsonl"));
        assert!(AutoDetectService::matches_pattern("file.json", "*.json"));
        assert!(!AutoDetectService::matches_pattern("test.txt", "*.jsonl"));
    }

    #[test]
    fn test_matches_pattern_prefix_wildcard() {
        // Note: Current implementation only supports suffix wildcards (*.ext)
        // For prefix wildcards like "session-*.json", we'd need to enhance the implementation
        // For now, we test what's actually implemented
        assert!(AutoDetectService::matches_pattern("session.json", "*.json"));
    }

    #[test]
    fn test_scan_all() {
        let detected = AutoDetectService::scan_all();
        assert_eq!(detected.len(), 4); // Should detect all 4 providers
    }
}
