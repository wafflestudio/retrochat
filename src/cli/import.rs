use anyhow::{Context, Result};
use std::path::Path;
use std::sync::Arc;

use crate::cli::help;
use crate::database::DatabaseManager;
use crate::models::provider::config::{
    ClaudeCodeConfig, CodexConfig, CursorAgentConfig, GeminiCliConfig,
};
use crate::models::Provider;
use crate::services::ImportService;

pub async fn handle_import_command(
    path: Option<String>,
    providers: Vec<Provider>,
    overwrite: bool,
) -> Result<()> {
    // Check if user provided a path
    if let Some(path_str) = path {
        return import_path(path_str, overwrite).await;
    }

    // Check if any providers are specified
    if !providers.is_empty() {
        return import_providers(providers, overwrite).await;
    }

    // No arguments provided - show help message
    help::print_import_usage();
    Err(anyhow::anyhow!("No import source specified"))
}

async fn import_path(path_str: String, overwrite: bool) -> Result<()> {
    let path = Path::new(&path_str);

    if !path.exists() {
        return Err(anyhow::anyhow!("Path does not exist: {path_str}"));
    }

    if path.is_file() {
        import_file(path_str, overwrite).await
    } else if path.is_dir() {
        import_batch(path_str, overwrite).await
    } else {
        Err(anyhow::anyhow!(
            "Path is neither a file nor a directory: {path_str}"
        ))
    }
}

async fn import_providers(providers: Vec<Provider>, overwrite: bool) -> Result<()> {
    // Expand "All" to all specific providers
    let expanded_providers = Provider::expand_all(providers);

    let mut imported_any = false;

    for provider in expanded_providers {
        match provider {
            Provider::All => {
                // Should not happen due to expansion above, but handle it anyway
                unreachable!("Provider::All should have been expanded")
            }
            Provider::ClaudeCode => {
                println!("Importing from Claude Code directories...");
                if let Err(e) = ClaudeCodeConfig::import_directories(overwrite, |path, ow| {
                    Box::pin(import_batch(path, ow))
                })
                .await
                {
                    eprintln!("Error importing Claude directories: {e}");
                } else {
                    imported_any = true;
                }
                println!();
            }
            Provider::GeminiCLI => {
                println!("Importing from Gemini directories...");
                if let Err(e) = GeminiCliConfig::import_directories(overwrite, |path, ow| {
                    Box::pin(import_batch(path, ow))
                })
                .await
                {
                    eprintln!("Error importing Gemini directories: {e}");
                } else {
                    imported_any = true;
                }
                println!();
            }
            Provider::Codex => {
                println!("Importing from Codex directories...");
                if let Err(e) = CodexConfig::import_directories(overwrite, |path, ow| {
                    Box::pin(import_batch(path, ow))
                })
                .await
                {
                    eprintln!("Error importing Codex directories: {e}");
                } else {
                    imported_any = true;
                }
                println!();
            }
            Provider::CursorAgent => {
                println!("Importing from Cursor directories...");
                if let Err(e) = CursorAgentConfig::import_directories(overwrite, |path, ow| {
                    Box::pin(import_batch(path, ow))
                })
                .await
                {
                    eprintln!("Error importing Cursor directories: {e}");
                } else {
                    imported_any = true;
                }
                println!();
            }
            Provider::Other(name) => {
                eprintln!("Unknown provider: {name}");
            }
        }
    }

    if imported_any {
        Ok(())
    } else {
        Err(anyhow::anyhow!("No providers were successfully imported"))
    }
}

async fn import_file(file_path: String, overwrite: bool) -> Result<()> {
    let path = Path::new(&file_path);

    println!("Importing file: {}", path.display());

    let db_path = crate::database::config::get_default_db_path()?;
    let db_manager = Arc::new(DatabaseManager::new(&db_path).await?);
    let import_service = ImportService::new(db_manager);

    // Detect provider
    let provider = crate::parsers::ParserRegistry::detect_provider(path)
        .ok_or_else(|| anyhow::anyhow!("Unsupported file format: {file_path}"))?;

    println!("Detected format: {provider}");

    if overwrite {
        println!("Overwrite mode: Will replace existing sessions");
    }

    let import_request = crate::services::ImportFileRequest {
        file_path: file_path.clone(),
        provider: Some(provider.to_string()),
        project_name: None,
        overwrite_existing: Some(overwrite),
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

async fn import_batch(directory: String, overwrite: bool) -> Result<()> {
    let path = Path::new(&directory);

    println!("Batch importing from directory: {}", path.display());

    if overwrite {
        println!("Overwrite mode: Will replace existing sessions");
    }

    let db_path = crate::database::config::get_default_db_path()?;
    let db_manager = Arc::new(DatabaseManager::new(&db_path).await?);
    let import_service = ImportService::new(db_manager);

    let batch_request = crate::services::BatchImportRequest {
        directory_path: directory.clone(),
        providers: None,
        project_name: None,
        overwrite_existing: Some(overwrite),
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
