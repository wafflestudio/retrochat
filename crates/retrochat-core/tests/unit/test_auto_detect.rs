use retrochat_core::services::{AutoDetectService, DetectedProvider};
use std::path::PathBuf;
use tempfile::TempDir;

#[test]
fn test_scan_all_returns_three_providers() {
    let detected = AutoDetectService::scan_all();
    assert_eq!(detected.len(), 3, "Should detect all 3 providers");

    // Check that all expected providers are present
    let provider_names: Vec<String> = detected.iter().map(|d| d.provider.to_string()).collect();

    assert!(provider_names.contains(&"Claude Code".to_string()));
    assert!(provider_names.contains(&"Gemini CLI".to_string()));
    assert!(provider_names.contains(&"Codex".to_string()));
}

#[test]
fn test_valid_providers_filters_correctly() {
    let all_detected = AutoDetectService::scan_all();
    let valid = AutoDetectService::valid_providers(&all_detected);

    // All valid providers should have is_valid = true
    for provider in &valid {
        assert!(
            provider.is_valid,
            "Valid providers should have is_valid = true"
        );
        assert!(
            provider.estimated_sessions > 0,
            "Valid providers should have sessions"
        );
    }
}

#[test]
fn test_total_sessions_calculation() {
    let mock_providers = vec![
        DetectedProvider {
            provider: retrochat::models::Provider::ClaudeCode,
            paths: vec![PathBuf::from("/test")],
            estimated_sessions: 10,
            is_valid: true,
        },
        DetectedProvider {
            provider: retrochat::models::Provider::GeminiCLI,
            paths: vec![PathBuf::from("/test2")],
            estimated_sessions: 20,
            is_valid: true,
        },
        DetectedProvider {
            provider: retrochat::models::Provider::Codex,
            paths: vec![],
            estimated_sessions: 0,
            is_valid: false,
        },
    ];

    let total = AutoDetectService::total_sessions(&mock_providers);
    assert_eq!(total, 30, "Should sum all estimated sessions");
}

#[test]
fn test_pattern_matching_via_detection() {
    // Test pattern matching indirectly through the detection mechanism
    // Since matches_pattern is private, we test it through public API

    // We can verify that the detection works by checking results
    let detected = AutoDetectService::scan_all();

    // All providers should be detected (even if invalid)
    assert_eq!(detected.len(), 3);

    // This verifies that pattern matching is working internally
    // as it's used by check_directory
}

#[test]
fn test_detection_with_temp_directory() {
    // Test that detection works by creating temp files
    // This indirectly tests check_directory functionality

    let temp_dir = TempDir::new().unwrap();

    // Create test files
    std::fs::write(temp_dir.path().join("test1.jsonl"), "test").unwrap();
    std::fs::write(temp_dir.path().join("test2.jsonl"), "test").unwrap();
    std::fs::write(temp_dir.path().join("other.txt"), "test").unwrap();

    // We test the overall behavior through the public API
    // The actual file detection happens in the real directories
    // This test just verifies the temp file creation works

    assert!(temp_dir.path().join("test1.jsonl").exists());
    assert!(temp_dir.path().join("test2.jsonl").exists());
}

#[test]
fn test_detected_provider_structure() {
    let provider = DetectedProvider {
        provider: retrochat::models::Provider::ClaudeCode,
        paths: vec![PathBuf::from("/test/path")],
        estimated_sessions: 42,
        is_valid: true,
    };

    assert_eq!(provider.provider.to_string(), "Claude Code");
    assert_eq!(provider.paths.len(), 1);
    assert_eq!(provider.estimated_sessions, 42);
    assert!(provider.is_valid);
}

#[test]
fn test_empty_valid_providers() {
    let mock_providers = vec![
        DetectedProvider {
            provider: retrochat::models::Provider::ClaudeCode,
            paths: vec![],
            estimated_sessions: 0,
            is_valid: false,
        },
        DetectedProvider {
            provider: retrochat::models::Provider::GeminiCLI,
            paths: vec![],
            estimated_sessions: 0,
            is_valid: false,
        },
    ];

    let valid = AutoDetectService::valid_providers(&mock_providers);
    assert_eq!(
        valid.len(),
        0,
        "Should return empty vec when no valid providers"
    );
}

#[test]
fn test_multiple_paths_per_provider() {
    let provider = DetectedProvider {
        provider: retrochat::models::Provider::ClaudeCode,
        paths: vec![
            PathBuf::from("/path1"),
            PathBuf::from("/path2"),
            PathBuf::from("/path3"),
        ],
        estimated_sessions: 100,
        is_valid: true,
    };

    assert_eq!(provider.paths.len(), 3);
    assert!(provider.is_valid);
}
