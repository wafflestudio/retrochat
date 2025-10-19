/// Centralized help and usage information for the CLI
///
/// This module provides consistent help messages across different CLI commands.
/// It uses the ProviderRegistry to get provider information dynamically.
use crate::models::provider::ProviderRegistry;
use crate::models::Provider;

// Helper: list providers supported via CLI (excluding `All` aggregate)
fn supported_providers() -> Vec<Provider> {
    ProviderRegistry::supported_providers()
}

// Helper: get provider information from registry
fn get_provider_info(p: &Provider) -> Option<(String, Option<String>, Option<String>)> {
    let registry = ProviderRegistry::new();
    registry.get_provider(p).map(|config| {
        (
            config.description().to_string(),
            config.env_var_name().map(|s| s.to_string()),
            config.default_directory().map(|s| s.to_string()),
        )
    })
}

// Helper: map Provider to human-friendly description
fn provider_description(p: &Provider) -> String {
    if matches!(p, Provider::All) {
        return "All providers".to_string();
    }

    get_provider_info(p)
        .map(|(desc, _, _)| desc)
        .unwrap_or_else(|| "Unknown provider".to_string())
}

// Helper: map Provider to environment variable name (if any)
fn provider_env_var(p: &Provider) -> Option<String> {
    if matches!(p, Provider::All | Provider::Other(_)) {
        return None;
    }

    get_provider_info(p).and_then(|(_, env_var, _)| env_var)
}

// Helper: map Provider to a default directory hint (if any)
fn provider_default_dir(p: &Provider) -> Option<String> {
    if matches!(p, Provider::All | Provider::Other(_)) {
        return None;
    }

    get_provider_info(p).and_then(|(_, _, default_dir)| default_dir)
}

/// Print getting started guide with import examples
pub fn print_getting_started() {
    println!();
    println!("Next steps:");
    print_import_help();
    println!("  2. Launch the TUI interface:");
    println!("     $ retrochat tui");
    println!();
    println!("  3. Generate insights:");
    println!("     $ retrochat analyze insights");
}

/// Print comprehensive import command usage (for command errors)
pub fn print_import_usage() {
    eprintln!("Usage: retrochat import [OPTIONS] [PROVIDERS]...");
    eprintln!();
    eprintln!("Import chat histories from LLM providers or specific paths");
    eprintln!();
    eprintln!("Options:");
    eprintln!("  --path <PATH>      Import from a specific file or directory");
    eprintln!("  --overwrite        Overwrite existing sessions");
    eprintln!();
    eprintln!("Available Providers:");
    eprintln!("  all                Import from all configured providers");
    for provider in supported_providers() {
        let arg = format_provider_arg(&provider);
        let desc = provider_description(&provider);
        eprintln!("  {arg:<18} {desc}");
    }
    eprintln!();
    eprintln!("Examples:");
    let args: Vec<String> = supported_providers()
        .into_iter()
        .take(2)
        .map(|p| format_provider_arg(&p))
        .collect();
    eprintln!(
        "  $ retrochat import {}           # Import from multiple providers",
        args.join(" ")
    );
    eprintln!("  $ retrochat import all                     # Import from all providers");
    eprintln!("  $ retrochat import --path ~/.claude/projects");
    if let Some(first_provider) = supported_providers().first() {
        eprintln!(
            "  $ retrochat import {} --overwrite      # Overwrite existing sessions",
            format_provider_arg(first_provider)
        );
    }
    eprintln!();
    eprintln!("Environment Variables:");
    for provider in supported_providers() {
        if let Some(env_var) = provider_env_var(&provider) {
            let desc = provider_description(&provider);
            if let Some(default_path) = provider_default_dir(&provider) {
                eprintln!("  {env_var:<23} - {desc} (default: {default_path})");
            } else {
                eprintln!("  {env_var:<23} - {desc} (no default)");
            }
        }
    }
    eprintln!();
    eprintln!("Note: Use colon (:) to separate multiple directories in environment variables");
}

/// Print import command usage examples
pub fn print_import_help() {
    println!("  1. Import your chat files:");
    println!();
    println!("     From provider directories:");
    for provider in &supported_providers() {
        // Provider enum uses lower-case value names in CLI (e.g., "gemini", "cursor")
        println!(
            "       $ retrochat import {}",
            format_provider_arg(provider)
        );
    }
    println!();
    println!("     Multiple providers at once:");
    println!(
        "       $ retrochat import {}",
        format_provider_arg(&Provider::All)
    );
    let args: Vec<String> = supported_providers()
        .into_iter()
        .take(3)
        .map(|p| format_provider_arg(&p))
        .collect();
    println!("       $ retrochat import {}", args.join(" "));
    println!();
    println!("     From a specific path:");
    println!("       $ retrochat import --path <file-or-directory>");
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
            let desc = provider_description(&provider);
            if let Some(default_path) = provider_default_dir(&provider) {
                println!("  {env_var:<25} - {desc} (default: {default_path})");
            } else {
                println!("  {env_var:<25} - {desc} (no default, must be configured)");
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
    println!("   $ retrochat tui");
    println!();
    println!("4. Generate insights:");
    println!("   $ retrochat analyze insights");
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
