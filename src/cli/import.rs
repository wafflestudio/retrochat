use anyhow::{Context, Result};
use std::env;
use std::path::Path;
use std::sync::Arc;

use crate::database::DatabaseManager;
use crate::services::ImportService;

pub async fn scan_directory(directory: Option<String>) -> Result<()> {
    let scan_path = directory.unwrap_or_else(|| ".".to_string());
    let path = Path::new(&scan_path);

    if !path.exists() {
        return Err(anyhow::anyhow!("Directory does not exist: {scan_path}"));
    }

    println!("Scanning directory: {}", path.display());

    let db_manager = Arc::new(DatabaseManager::new("retrochat.db").await?);
    let import_service = ImportService::new(db_manager);

    let scan_request = crate::services::ScanRequest {
        directory_path: scan_path.clone(),
        providers: None,
        recursive: Some(true),
    };

    let scan_response = import_service
        .scan_directory(scan_request)
        .await
        .with_context(|| format!("Failed to scan directory: {}", path.display()))?;

    if scan_response.files_found.is_empty() {
        println!("No supported chat files found in directory");
        return Ok(());
    }

    println!(
        "Found {} supported chat files:",
        scan_response.files_found.len()
    );
    for file in &scan_response.files_found {
        println!("  {} ({})", file.file_path, file.provider);
    }

    println!("\nSupported formats:");
    for provider in crate::parsers::ParserRegistry::get_supported_providers() {
        println!("  - {provider}");
    }

    println!("\nCommon chat directories:");
    println!("  - ~/.claude/projects (Claude Code)");
    println!("  - ~/.gemini/tmp (Gemini)");

    println!("\nProvider-specific commands:");
    println!("  retrochat import scan-claude  - Scan Claude directories");
    println!("  retrochat import scan-gemini  - Scan Gemini directories");
    println!("  retrochat import scan-codex   - Scan Codex directories");

    println!("\nUse 'retrochat import file <path>' to import a specific file");
    println!("Use 'retrochat import batch <directory>' to import all files in a directory");

    Ok(())
}

pub async fn import_file(file_path: String) -> Result<()> {
    let path = Path::new(&file_path);

    if !path.exists() {
        return Err(anyhow::anyhow!("File does not exist: {file_path}"));
    }

    println!("Importing file: {}", path.display());

    let db_manager = Arc::new(DatabaseManager::new("retrochat.db").await?);
    let import_service = ImportService::new(db_manager);

    // Detect provider
    let provider = crate::parsers::ParserRegistry::detect_provider(path)
        .ok_or_else(|| anyhow::anyhow!("Unsupported file format: {file_path}"))?;

    println!("Detected format: {provider}");

    let import_request = crate::services::ImportFileRequest {
        file_path: file_path.clone(),
        provider: Some(provider.to_string()),
        project_name: None,
        overwrite_existing: None,
    };

    let import_response = import_service
        .import_file(import_request)
        .await
        .with_context(|| format!("Failed to import file: {file_path}"))?;

    println!("Import completed:");
    println!(
        "  - {} sessions imported",
        import_response.sessions_imported
    );
    println!(
        "  - {} messages imported",
        import_response.messages_imported
    );

    if !import_response.warnings.is_empty() {
        println!("Warnings:");
        for warning in &import_response.warnings {
            println!("  - {warning}");
        }
    }

    Ok(())
}

pub async fn import_batch(directory: String) -> Result<()> {
    let path = Path::new(&directory);

    if !path.exists() {
        return Err(anyhow::anyhow!("Directory does not exist: {directory}"));
    }

    println!("Batch importing from directory: {}", path.display());

    let db_manager = Arc::new(DatabaseManager::new("retrochat.db").await?);
    let import_service = ImportService::new(db_manager);

    let batch_request = crate::services::BatchImportRequest {
        directory_path: directory.clone(),
        providers: None,
        project_name: None,
        overwrite_existing: None,
        recursive: Some(true),
    };

    let batch_response = import_service
        .import_batch(batch_request)
        .await
        .with_context(|| format!("Failed to batch import from directory: {}", path.display()))?;

    println!("\nBatch import completed:");
    println!(
        "  - {} files processed",
        batch_response.total_files_processed
    );
    println!(
        "  - {} files imported successfully",
        batch_response.successful_imports
    );
    println!(
        "  - {} sessions imported",
        batch_response.total_sessions_imported
    );
    println!(
        "  - {} messages imported",
        batch_response.total_messages_imported
    );

    if batch_response.failed_imports > 0 {
        println!(
            "  - {} files failed to import",
            batch_response.failed_imports
        );
        if !batch_response.errors.is_empty() {
            println!("Errors:");
            for error in &batch_response.errors {
                println!("  - {error}");
            }
        }
    }

    Ok(())
}

pub async fn handle_scan_command(directory: Option<String>) -> Result<()> {
    match directory {
        Some(dir) => scan_directory(Some(dir)).await,
        None => scan_enabled_providers().await,
    }
}

async fn scan_enabled_providers() -> Result<()> {
    println!("Scanning enabled AI service directories...\n");

    let claude_enabled = env::var("RETROCHAT_ENABLE_CLAUDE")
        .unwrap_or_else(|_| "true".to_string())
        .parse::<bool>()
        .unwrap_or(true);

    let gemini_enabled = env::var("RETROCHAT_ENABLE_GEMINI")
        .unwrap_or_else(|_| "true".to_string())
        .parse::<bool>()
        .unwrap_or(true);

    let codex_enabled = env::var("RETROCHAT_ENABLE_CODEX")
        .unwrap_or_else(|_| "false".to_string())
        .parse::<bool>()
        .unwrap_or(false);

    let mut scanned_any = false;

    if claude_enabled {
        scan_claude_directories().await?;
        scanned_any = true;
    }

    if gemini_enabled {
        scan_gemini_directories().await?;
        scanned_any = true;
    }

    if codex_enabled {
        scan_codex_directories().await?;
        scanned_any = true;
    }

    if !scanned_any {
        println!("No AI services are enabled. Check your .env configuration:");
        println!("  RETROCHAT_ENABLE_CLAUDE=true");
        println!("  RETROCHAT_ENABLE_GEMINI=true");
        println!("  RETROCHAT_ENABLE_CODEX=true");
        println!("\nOr scan a specific directory with: retrochat import scan <directory>");
    }

    Ok(())
}

pub async fn handle_import_file_command(file_path: String) -> Result<()> {
    import_file(file_path).await
}

pub async fn handle_import_batch_command(directory: String) -> Result<()> {
    import_batch(directory).await
}

pub async fn scan_claude_directories() -> Result<()> {
    let claude_enabled = env::var("RETROCHAT_ENABLE_CLAUDE")
        .unwrap_or_else(|_| "true".to_string())
        .parse::<bool>()
        .unwrap_or(true);

    if !claude_enabled {
        println!("Claude scanning is disabled. Set RETROCHAT_ENABLE_CLAUDE=true to enable.");
        return Ok(());
    }

    let home = env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let claude_dirs =
        env::var("RETROCHAT_CLAUDE_DIRS").unwrap_or_else(|_| format!("{home}/.claude/projects"));

    println!("Scanning Claude directories:");
    for dir_str in claude_dirs.split(':') {
        if dir_str.is_empty() {
            continue;
        }

        let dir_path = if dir_str.starts_with('~') {
            dir_str.replacen('~', &home, 1)
        } else {
            dir_str.to_string()
        };

        let path = Path::new(&dir_path);
        if path.exists() {
            println!("  Scanning: {}", path.display());
            scan_directory(Some(dir_path)).await?;
        } else {
            println!("  Directory not found: {}", path.display());
        }
    }

    Ok(())
}

pub async fn scan_gemini_directories() -> Result<()> {
    let gemini_enabled = env::var("RETROCHAT_ENABLE_GEMINI")
        .unwrap_or_else(|_| "true".to_string())
        .parse::<bool>()
        .unwrap_or(true);

    if !gemini_enabled {
        println!("Gemini scanning is disabled. Set RETROCHAT_ENABLE_GEMINI=true to enable.");
        return Ok(());
    }

    let gemini_dirs = env::var("RETROCHAT_GEMINI_DIRS").unwrap_or_else(|_| "".to_string());

    if gemini_dirs.trim().is_empty() {
        println!(
            "No Gemini directories configured. Set RETROCHAT_GEMINI_DIRS environment variable."
        );
        return Ok(());
    }

    println!("Scanning Gemini directories:");
    for dir_str in gemini_dirs.split(':') {
        if dir_str.is_empty() {
            continue;
        }

        let home = env::var("HOME").unwrap_or_else(|_| ".".to_string());
        let dir_path = if dir_str.starts_with('~') {
            dir_str.replacen('~', &home, 1)
        } else {
            dir_str.to_string()
        };

        let path = Path::new(&dir_path);
        if path.exists() {
            println!("  Scanning: {}", path.display());
            scan_directory(Some(dir_path)).await?;
        } else {
            println!("  Directory not found: {}", path.display());
        }
    }

    Ok(())
}

pub async fn scan_codex_directories() -> Result<()> {
    let codex_enabled = env::var("RETROCHAT_ENABLE_CODEX")
        .unwrap_or_else(|_| "false".to_string())
        .parse::<bool>()
        .unwrap_or(false);

    if !codex_enabled {
        println!("Codex scanning is disabled. Set RETROCHAT_ENABLE_CODEX=true to enable.");
        return Ok(());
    }

    let codex_dirs = env::var("RETROCHAT_CODEX_DIRS").unwrap_or_else(|_| "".to_string());

    if codex_dirs.trim().is_empty() {
        println!("No Codex directories configured. Set RETROCHAT_CODEX_DIRS environment variable.");
        return Ok(());
    }

    println!("Scanning Codex directories:");
    for dir_str in codex_dirs.split(':') {
        if dir_str.is_empty() {
            continue;
        }

        let home = env::var("HOME").unwrap_or_else(|_| ".".to_string());
        let dir_path = if dir_str.starts_with('~') {
            dir_str.replacen('~', &home, 1)
        } else {
            dir_str.to_string()
        };

        let path = Path::new(&dir_path);
        if path.exists() {
            println!("  Scanning: {}", path.display());
            scan_directory(Some(dir_path)).await?;
        } else {
            println!("  Directory not found: {}", path.display());
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_scan_directory() {
        let temp_dir = TempDir::new().unwrap();
        eprintln!("temp dir: {}", temp_dir.path().display());

        // Create test files
        let claude_file = temp_dir.path().join("claude.jsonl");
        fs::write(
            &claude_file,
            r#"{"uuid":"550e8400-e29b-41d4-a716-446655440000","chat_messages":[]}"#,
        )
        .unwrap();
        eprintln!("before scan, ls: {:?}", std::fs::read_dir(temp_dir.path()).unwrap()
        .map(|e| e.unwrap().path()).collect::<Vec<_>>());

        let result = scan_directory(Some(temp_dir.path().to_string_lossy().to_string())).await;
        eprintln!("result: {:?}", result);
        assert!(result.is_ok());
    }
}
