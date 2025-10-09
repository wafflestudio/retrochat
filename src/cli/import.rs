use anyhow::{Context, Result};
use std::env;
use std::path::Path;
use std::sync::Arc;

use crate::database::DatabaseManager;
use crate::services::ImportService;

pub async fn handle_import_command(
    path: Option<String>,
    claude: bool,
    gemini: bool,
    codex: bool,
    cursor: bool,
    overwrite: bool,
) -> Result<()> {
    // Check if user provided a path
    if let Some(path_str) = path {
        return import_path(path_str, overwrite).await;
    }

    // Check if any provider flags are set
    if claude || gemini || codex || cursor {
        return import_providers(claude, gemini, codex, cursor, overwrite).await;
    }

    // No arguments provided - show help message
    Err(anyhow::anyhow!(
        "No import source specified. Use --path to import from a specific location or use provider flags like --claude, --gemini, etc."
    ))
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
        Err(anyhow::anyhow!("Path is neither a file nor a directory: {path_str}"))
    }
}

async fn import_providers(
    claude: bool,
    gemini: bool,
    codex: bool,
    cursor: bool,
    overwrite: bool,
) -> Result<()> {
    let mut imported_any = false;

    if claude {
        println!("Importing from Claude Code directories...");
        if let Err(e) = import_claude_directories(overwrite).await {
            eprintln!("Error importing Claude directories: {e}");
        } else {
            imported_any = true;
        }
        println!();
    }

    if gemini {
        println!("Importing from Gemini directories...");
        if let Err(e) = import_gemini_directories(overwrite).await {
            eprintln!("Error importing Gemini directories: {e}");
        } else {
            imported_any = true;
        }
        println!();
    }

    if codex {
        println!("Importing from Codex directories...");
        if let Err(e) = import_codex_directories(overwrite).await {
            eprintln!("Error importing Codex directories: {e}");
        } else {
            imported_any = true;
        }
        println!();
    }

    if cursor {
        println!("Importing from Cursor directories...");
        if let Err(e) = import_cursor_directories(overwrite).await {
            eprintln!("Error importing Cursor directories: {e}");
        } else {
            imported_any = true;
        }
        println!();
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

    let db_manager = Arc::new(DatabaseManager::new("retrochat.db").await?);
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

    let db_manager = Arc::new(DatabaseManager::new("retrochat.db").await?);
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

async fn import_claude_directories(overwrite: bool) -> Result<()> {
    let claude_enabled = env::var("RETROCHAT_ENABLE_CLAUDE")
        .unwrap_or_else(|_| "true".to_string())
        .parse::<bool>()
        .unwrap_or(true);

    if !claude_enabled {
        println!("Claude import is disabled. Set RETROCHAT_ENABLE_CLAUDE=true to enable.");
        return Ok(());
    }

    let home = env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let claude_dirs =
        env::var("RETROCHAT_CLAUDE_DIRS").unwrap_or_else(|_| format!("{home}/.claude/projects"));

    let mut imported_any = false;

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
            println!("  Importing from: {}", path.display());
            if let Err(e) = import_batch(dir_path, overwrite).await {
                eprintln!("  Error: {e}");
            } else {
                imported_any = true;
            }
        } else {
            println!("  Directory not found: {}", path.display());
        }
    }

    if !imported_any {
        println!("  No Claude directories found or imported");
    }

    Ok(())
}

async fn import_gemini_directories(overwrite: bool) -> Result<()> {
    let gemini_enabled = env::var("RETROCHAT_ENABLE_GEMINI")
        .unwrap_or_else(|_| "true".to_string())
        .parse::<bool>()
        .unwrap_or(true);

    if !gemini_enabled {
        println!("Gemini import is disabled. Set RETROCHAT_ENABLE_GEMINI=true to enable.");
        return Ok(());
    }

    let gemini_dirs = env::var("RETROCHAT_GEMINI_DIRS").unwrap_or_else(|_| "".to_string());

    if gemini_dirs.trim().is_empty() {
        println!("  No Gemini directories configured. Set RETROCHAT_GEMINI_DIRS environment variable.");
        return Ok(());
    }

    let mut imported_any = false;

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
            println!("  Importing from: {}", path.display());
            if let Err(e) = import_batch(dir_path, overwrite).await {
                eprintln!("  Error: {e}");
            } else {
                imported_any = true;
            }
        } else {
            println!("  Directory not found: {}", path.display());
        }
    }

    if !imported_any {
        println!("  No Gemini directories found or imported");
    }

    Ok(())
}

async fn import_codex_directories(overwrite: bool) -> Result<()> {
    let codex_enabled = env::var("RETROCHAT_ENABLE_CODEX")
        .unwrap_or_else(|_| "false".to_string())
        .parse::<bool>()
        .unwrap_or(false);

    if !codex_enabled {
        println!("Codex import is disabled. Set RETROCHAT_ENABLE_CODEX=true to enable.");
        return Ok(());
    }

    let codex_dirs = env::var("RETROCHAT_CODEX_DIRS").unwrap_or_else(|_| "".to_string());

    if codex_dirs.trim().is_empty() {
        println!("  No Codex directories configured. Set RETROCHAT_CODEX_DIRS environment variable.");
        return Ok(());
    }

    let mut imported_any = false;

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
            println!("  Importing from: {}", path.display());
            if let Err(e) = import_batch(dir_path, overwrite).await {
                eprintln!("  Error: {e}");
            } else {
                imported_any = true;
            }
        } else {
            println!("  Directory not found: {}", path.display());
        }
    }

    if !imported_any {
        println!("  No Codex directories found or imported");
    }

    Ok(())
}

async fn import_cursor_directories(overwrite: bool) -> Result<()> {
    let cursor_enabled = env::var("RETROCHAT_ENABLE_CURSOR")
        .unwrap_or_else(|_| "true".to_string())
        .parse::<bool>()
        .unwrap_or(true);

    if !cursor_enabled {
        println!("Cursor import is disabled. Set RETROCHAT_ENABLE_CURSOR=true to enable.");
        return Ok(());
    }

    let home = env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let cursor_dirs = env::var("RETROCHAT_CURSOR_DIRS")
        .unwrap_or_else(|_| format!("{home}/.cursor/chats"));

    let mut imported_any = false;

    for dir_str in cursor_dirs.split(':') {
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
            println!("  Importing from: {}", path.display());
            if let Err(e) = import_batch(dir_path, overwrite).await {
                eprintln!("  Error: {e}");
            } else {
                imported_any = true;
            }
        } else {
            println!("  Directory not found: {}", path.display());
        }
    }

    if !imported_any {
        println!("  No Cursor directories found or imported");
    }

    Ok(())
}
