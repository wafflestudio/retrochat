/// Centralized help and usage information for the CLI
///
/// This module provides consistent help messages across different CLI commands.
/// It now uses the `Provider` enum from the models module to drive examples and
/// supported provider listings.
use crate::models::Provider;

// Helper: list providers supported via CLI (excluding `All` aggregate)
fn supported_providers() -> Vec<Provider> {
    Provider::all_concrete()
}

// Helper: map Provider to human-friendly description
fn provider_description(p: &Provider) -> &'static str {
    match p {
        Provider::ClaudeCode => "Claude Code (.jsonl files)",
        Provider::GeminiCLI => "Gemini CLI (.json files)",
        Provider::Codex => "Codex (various formats)",
        Provider::CursorAgent => "Cursor Agent (store.db files)",
        Provider::All => "All providers",
        Provider::Other(_) => "Unknown provider",
    }
}

// Helper: map Provider to environment variable name (if any)
fn provider_env_var(p: &Provider) -> Option<&'static str> {
    match p {
        Provider::ClaudeCode => Some("RETROCHAT_CLAUDE_DIRS"),
        Provider::GeminiCLI => Some("RETROCHAT_GEMINI_DIRS"),
        Provider::Codex => Some("RETROCHAT_CODEX_DIRS"),
        Provider::CursorAgent => Some("RETROCHAT_CURSOR_DIRS"),
        Provider::All | Provider::Other(_) => None,
    }
}

// Helper: map Provider to a default directory hint (if any)
fn provider_default_dir(p: &Provider) -> Option<&'static str> {
    match p {
        Provider::ClaudeCode => Some("~/.claude/projects"),
        Provider::GeminiCLI => Some("~/.gemini/tmp"),
        Provider::Codex => Some("~/.codex/sessions"),
        Provider::CursorAgent => Some("~/.cursor/chats"),
        Provider::All | Provider::Other(_) => None,
    }
}

/// Print getting started guide with import examples
pub fn print_getting_started() {
    println!();
    println!("Next steps:");
    print_import_help();
    println!("  2. Launch the TUI interface:");
    println!("     retrochat tui");
    println!();
    println!("  3. Generate insights:");
    println!("     retrochat analyze insights");
}

/// Print import command usage examples
pub fn print_import_help() {
    println!("  1. Import your chat files:");
    println!();
    println!("     From provider directories:");
    for provider in &supported_providers() {
        // Provider enum uses lower-case value names in CLI (e.g., "gemini", "cursor")
        println!("       retrochat import {}", format_provider_arg(provider));
    }
    println!();
    println!("     Multiple providers at once:");
    let args: Vec<String> = supported_providers()
        .into_iter()
        .take(3)
        .map(|p| format_provider_arg(&p))
        .collect();
    println!("       retrochat import {}", args.join(" "));
    println!();
    println!("     From a specific path:");
    println!("       retrochat import --path <file-or-directory>");
    println!();
}

/// Print supported file formats
pub fn print_supported_formats() {
    println!("Supported formats:");
    for provider in supported_providers() {
        println!("  • {}", provider_description(&provider));
    }
    println!();
}

/// Print environment variable configuration
pub fn print_environment_config() {
    println!("Environment Variables:");
    for provider in supported_providers() {
        if let Some(env_var) = provider_env_var(&provider) {
            if let Some(default_path) = provider_default_dir(&provider) {
                println!(
                    "  {:<25} - {} (default: {})",
                    env_var,
                    provider_description(&provider),
                    default_path
                );
            } else {
                println!(
                    "  {:<25} - {} (no default, must be configured)",
                    env_var,
                    provider_description(&provider)
                );
            }
        }
    }
    println!();
    println!("  Use colon (:) to separate multiple directories");
    println!("  Example: export RETROCHAT_CLAUDE_DIRS=\"~/.claude/projects:/other/path\"");
    println!();
}

/// Print full getting started guide for TUI
pub fn print_full_getting_started() {
    println!("Getting Started:");
    println!();
    print_import_help();
    print_supported_formats();
    println!("3. Launch TUI:");
    println!("   retrochat tui");
    println!();
    println!("4. Generate insights:");
    println!("   retrochat analyze insights");
    println!();
}

/// Print query command examples
pub fn print_query_examples() {
    println!("Query Commands:");
    println!("  retrochat query sessions                    - List all sessions");
    println!("  retrochat query sessions --provider claude  - Filter by provider");
    println!("  retrochat query session <SESSION_ID>        - View session details");
    println!("  retrochat query search <QUERY>              - Search messages");
    println!();
}

/// Print analyze command examples
pub fn print_analyze_examples() {
    println!("Analytics Commands:");
    println!("  retrochat analyze insights           - Generate usage insights");
    println!("  retrochat analyze export json        - Export to JSON");
    println!("  retrochat analyze export csv         - Export to CSV");
    println!("  retrochat analyze export txt         - Export to text");
    println!();
}

/// Print retrospection command examples
pub fn print_retrospect_examples() {
    println!("Retrospection Commands (requires GOOGLE_AI_API_KEY):");
    println!("  retrochat retrospect execute <SESSION_ID>           - Analyze a session");
    println!("  retrochat retrospect execute --all                  - Analyze all sessions");
    println!("  retrochat retrospect show <SESSION_ID>              - View analysis results");
    println!("  retrochat retrospect status                         - Check analysis status");
    println!("  retrochat retrospect cancel <REQUEST_ID>            - Cancel an analysis");
    println!();
}

// Helper: format a provider as it should appear in CLI examples
fn format_provider_arg(p: &Provider) -> String {
    match p {
        Provider::All => "all".to_string(),
        Provider::ClaudeCode => "claude".to_string(),
        Provider::GeminiCLI => "gemini".to_string(),
        Provider::Codex => "codex".to_string(),
        Provider::CursorAgent => "cursor".to_string(),
        Provider::Other(name) => name.clone(),
    }
}
