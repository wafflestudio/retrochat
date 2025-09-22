use anyhow::{Context, Result};
use std::path::Path;
use std::sync::Arc;

use crate::database::connection::DatabaseManager;
use crate::services::ImportService;

pub async fn scan_directory(directory: Option<String>) -> Result<()> {
    let scan_path = directory.unwrap_or_else(|| ".".to_string());
    let path = Path::new(&scan_path);

    if !path.exists() {
        return Err(anyhow::anyhow!("Directory does not exist: {scan_path}"));
    }

    println!("Scanning directory: {}", path.display());

    let db_manager = Arc::new(DatabaseManager::new("retrochat.db")?);
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

    let db_manager = Arc::new(DatabaseManager::new("retrochat.db")?);
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

    let db_manager = Arc::new(DatabaseManager::new("retrochat.db")?);
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
    scan_directory(directory).await
}

pub async fn handle_import_file_command(file_path: String) -> Result<()> {
    import_file(file_path).await
}

pub async fn handle_import_batch_command(directory: String) -> Result<()> {
    import_batch(directory).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_scan_directory() {
        let temp_dir = TempDir::new().unwrap();

        // Create test files
        let claude_file = temp_dir.path().join("claude.jsonl");
        fs::write(
            &claude_file,
            r#"{"uuid":"550e8400-e29b-41d4-a716-446655440000","chat_messages":[]}"#,
        )
        .unwrap();

        let result = scan_directory(Some(temp_dir.path().to_string_lossy().to_string())).await;
        assert!(result.is_ok());
    }
}
