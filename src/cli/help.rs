/// Centralized help and usage information for the CLI
///
/// This module provides consistent help messages across different CLI commands.
/// It uses the LlmProviderRegistry to ensure help text stays in sync
/// with the codebase.
///
/// ## Design Principles
///
/// 1. **Single Source of Truth**: All provider information (CLI names, descriptions,
///    environment variables, defaults) is defined in the `LlmProviderRegistry`.
///
/// 2. **Type Safety**: Uses actual provider configs from the registry, ensuring
///    all providers are properly configured before being shown in help.
///
/// 3. **Automatic Synchronization**: When you add a new provider to the registry's
///    `load_default_providers()`, all help messages automatically reflect the change.
///
/// ## Adding a New Provider
///
/// To add a new provider:
/// 1. Add the enum variant to `models::LlmProvider`
/// 2. Add provider configuration in `LlmProviderRegistry::load_default_providers()`
/// 3. Set cli_name, description, env_var_name, and default_directory in the config
///
/// The help system will automatically generate all relevant help messages.
///
use crate::models::LlmProviderRegistry;

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
    let registry = LlmProviderRegistry::new();
    let providers = registry.all_known();

    println!("  1. Import your chat files:");
    println!();
    println!("     From provider directories:");
    for provider in &providers {
        println!("       retrochat import --{}", provider.cli_name());
    }
    println!();
    println!("     Multiple providers at once:");
    let flags: Vec<String> = providers.iter()
        .take(3)
        .map(|p| format!("--{}", p.cli_name()))
        .collect();
    println!("       retrochat import {}", flags.join(" "));
    println!();
    println!("     From a specific path:");
    println!("       retrochat import --path <file-or-directory>");
    println!();
}

/// Print supported file formats
pub fn print_supported_formats() {
    let registry = LlmProviderRegistry::new();
    let providers = registry.all_known();

    println!("Supported formats:");
    for provider in providers {
        println!("  • {}", provider.description());
    }
    println!();
}

/// Print environment variable configuration
pub fn print_environment_config() {
    let registry = LlmProviderRegistry::new();
    let providers = registry.all_known();

    println!("Environment Variables:");
    for provider in providers {
        if let Some(env_var) = provider.env_var_name() {
            if let Some(default_path) = provider.default_directory() {
                println!("  {:<25} - {} (default: {})",
                    env_var, provider.description(), default_path);
            } else {
                println!("  {:<25} - {} (no default, must be configured)",
                    env_var, provider.description());
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
    println!("  retrochat query sessions --provider Claude  - Filter by provider");
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
