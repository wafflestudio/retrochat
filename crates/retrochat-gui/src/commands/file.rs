use crate::dto::{ImportFileResult, ImportSessionsResponse};
use crate::{AppState, OpenedFiles};
use retrochat_core::models::provider::config::{ClaudeCodeConfig, CodexConfig, GeminiCliConfig};
use retrochat_core::models::Provider;
use retrochat_core::services::{BatchImportRequest, ImportFileRequest};
use std::path::PathBuf;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager, State};
use tokio::sync::Mutex;

// Handler for file associations and drops
pub fn handle_file_drop(app: AppHandle, files: Vec<PathBuf>) {
    log::info!("handle_file_drop called with {} files", files.len());

    if files.is_empty() {
        log::debug!("No files to handle");
        return;
    }

    let file_paths: Vec<String> = files
        .into_iter()
        .filter_map(|path| {
            let path_str = path.to_str().map(|s| s.to_string());
            if let Some(ref p) = path_str {
                log::debug!("Processing file: {}", p);
            }
            path_str
        })
        .collect();

    log::info!("Converted {} file paths", file_paths.len());

    if let Some(opened_files) = app.try_state::<OpenedFiles>() {
        if let Ok(mut files) = opened_files.0.lock() {
            log::debug!("Updating opened files state");
            *files = file_paths.clone();
        }
    }

    // Emit event to frontend with the opened files
    log::debug!("Emitting file-opened event to frontend");
    match app.emit("file-opened", file_paths.clone()) {
        Ok(_) => log::info!("Successfully emitted file-opened event"),
        Err(e) => log::error!("Failed to emit file-opened event: {}", e),
    }
}

// Command to get opened files
#[tauri::command]
pub fn get_opened_files(state: State<OpenedFiles>) -> Vec<String> {
    log::debug!("get_opened_files called");
    let files = state.0.lock().unwrap().clone();
    log::debug!("Returning {} opened files", files.len());
    files
}

// Command to clear opened files
#[tauri::command]
pub fn clear_opened_files(state: State<OpenedFiles>) {
    log::debug!("clear_opened_files called");
    state.0.lock().unwrap().clear();
    log::info!("Cleared opened files");
}

// Command to import sessions from files
#[tauri::command]
pub async fn import_sessions(
    state: State<'_, Arc<Mutex<AppState>>>,
    file_paths: Vec<String>,
) -> Result<ImportSessionsResponse, String> {
    log::info!("import_sessions called with {} files", file_paths.len());

    let state_guard = state.lock().await;
    let import_service = &state_guard.import_service;

    let mut results = Vec::new();
    let mut total_sessions_imported = 0;
    let mut total_messages_imported = 0;
    let mut successful_imports = 0;
    let mut failed_imports = 0;

    // Import each file
    for (index, file_path) in file_paths.iter().enumerate() {
        log::info!(
            "Importing file {}/{}: {}",
            index + 1,
            file_paths.len(),
            file_path
        );

        let request = ImportFileRequest {
            file_path: file_path.clone(),
            provider: None, // Auto-detect via retrochat lib
            project_name: None,
            overwrite_existing: Some(false),
        };

        match import_service.import_file(request).await {
            Ok(response) => {
                log::info!(
                    "Successfully imported file '{}': {} sessions, {} messages",
                    file_path,
                    response.sessions_imported,
                    response.messages_imported
                );
                successful_imports += 1;
                total_sessions_imported += response.sessions_imported;
                total_messages_imported += response.messages_imported;

                results.push(ImportFileResult {
                    file_path: file_path.clone(),
                    sessions_imported: response.sessions_imported,
                    messages_imported: response.messages_imported,
                    success: true,
                    error: None,
                });
            }
            Err(e) => {
                log::error!("Failed to import file '{}': {}", file_path, e);
                failed_imports += 1;
                results.push(ImportFileResult {
                    file_path: file_path.clone(),
                    sessions_imported: 0,
                    messages_imported: 0,
                    success: false,
                    error: Some(e.to_string()),
                });
            }
        }
    }

    log::info!(
        "Import completed - {} successful, {} failed, total: {} sessions, {} messages",
        successful_imports,
        failed_imports,
        total_sessions_imported,
        total_messages_imported
    );

    Ok(ImportSessionsResponse {
        total_files: file_paths.len() as i32,
        successful_imports,
        failed_imports,
        total_sessions_imported,
        total_messages_imported,
        results,
    })
}

// Helper struct to track import stats
struct ImportStats {
    results: Vec<ImportFileResult>,
    total_sessions_imported: i32,
    total_messages_imported: i32,
    successful_imports: i32,
    failed_imports: i32,
    total_files: i32,
}

impl ImportStats {
    fn new() -> Self {
        Self {
            results: Vec::new(),
            total_sessions_imported: 0,
            total_messages_imported: 0,
            successful_imports: 0,
            failed_imports: 0,
            total_files: 0,
        }
    }
}

// Command to import sessions from preset providers
#[tauri::command]
pub async fn import_from_provider(
    state: State<'_, Arc<Mutex<AppState>>>,
    provider: String,
    overwrite: bool,
) -> Result<ImportSessionsResponse, String> {
    log::info!(
        "import_from_provider called with provider: {}, overwrite: {}",
        provider,
        overwrite
    );

    let state_guard = state.lock().await;
    let import_service = &state_guard.import_service;

    let mut stats = ImportStats::new();

    // Parse provider string to Provider enum
    let providers = match provider.to_lowercase().as_str() {
        "all" => vec![Provider::All],
        "claude" => vec![Provider::ClaudeCode],
        "gemini" => vec![Provider::GeminiCLI],
        "codex" => vec![Provider::Codex],
        "cursor-client" | "cursor client" => vec![Provider::CursorClient],
        _ => return Err(format!("Unknown provider: {}", provider)),
    };

    // Expand "All" to all specific providers
    let expanded_providers = Provider::expand_all(providers);

    for prov in expanded_providers {
        match prov {
            Provider::All => {
                // Should not happen due to expansion above
                unreachable!("Provider::All should have been expanded")
            }
            Provider::ClaudeCode => {
                log::info!("Importing from Claude Code directories...");
                if let Err(e) = import_provider_directories(
                    &ClaudeCodeConfig::create(),
                    import_service,
                    overwrite,
                    &mut stats,
                )
                .await
                {
                    log::error!("Error importing Claude directories: {}", e);
                }
            }
            Provider::GeminiCLI => {
                log::info!("Importing from Gemini directories...");
                if let Err(e) = import_provider_directories(
                    &GeminiCliConfig::create(),
                    import_service,
                    overwrite,
                    &mut stats,
                )
                .await
                {
                    log::error!("Error importing Gemini directories: {}", e);
                }
            }
            Provider::Codex => {
                log::info!("Importing from Codex directories...");
                if let Err(e) = import_provider_directories(
                    &CodexConfig::create(),
                    import_service,
                    overwrite,
                    &mut stats,
                )
                .await
                {
                    log::error!("Error importing Codex directories: {}", e);
                }
            }
            Provider::CursorClient => {
                log::info!("Importing from Cursor Client...");
                if let Some(workspace_path) =
                    retrochat_core::parsers::CursorClientParser::get_default_workspace_path()
                {
                    if let Some(parent) = workspace_path.parent() {
                        let global_db = parent.join("globalStorage/state.vscdb");
                        if global_db.exists() {
                            let request = retrochat_core::services::ImportFileRequest {
                                file_path: global_db.to_string_lossy().to_string(),
                                provider: Some("CursorClient".to_string()),
                                project_name: None,
                                overwrite_existing: Some(overwrite),
                            };
                            match import_service.import_file(request).await {
                                Ok(_) => {
                                    stats.successful_imports += 1;
                                }
                                Err(e) => {
                                    log::error!("Error importing Cursor data: {}", e);
                                    stats.failed_imports += 1;
                                }
                            }
                        }
                    }
                } else {
                    log::error!("Could not find Cursor workspace storage path");
                }
            }
            Provider::Other(name) => {
                log::error!("Unknown provider: {}", name);
                return Err(format!("Unknown provider: {}", name));
            }
        }
    }

    log::info!(
        "Provider import completed - {} successful, {} failed, total: {} sessions, {} messages",
        stats.successful_imports,
        stats.failed_imports,
        stats.total_sessions_imported,
        stats.total_messages_imported
    );

    Ok(ImportSessionsResponse {
        total_files: stats.total_files,
        successful_imports: stats.successful_imports,
        failed_imports: stats.failed_imports,
        total_sessions_imported: stats.total_sessions_imported,
        total_messages_imported: stats.total_messages_imported,
        results: stats.results,
    })
}

// Helper function to import from a provider's directories
async fn import_provider_directories(
    config: &retrochat_core::models::provider::config::ProviderConfig,
    import_service: &retrochat_core::services::ImportService,
    overwrite: bool,
    stats: &mut ImportStats,
) -> Result<(), String> {
    let directories = config.get_import_directories();

    if directories.is_empty() {
        log::info!("No directories found for provider: {}", config.name);
        return Ok(());
    }

    for dir_path in directories {
        let path = std::path::Path::new(&dir_path);
        if !path.exists() {
            log::warn!("Directory not found: {}", path.display());
            continue;
        }

        log::info!("Importing from directory: {}", path.display());

        let batch_request = BatchImportRequest {
            directory_path: dir_path.clone(),
            providers: None,
            project_name: None,
            overwrite_existing: Some(overwrite),
            recursive: Some(true),
        };

        match import_service.import_batch(batch_request).await {
            Ok(response) => {
                log::info!(
                    "Successfully imported from directory '{}': {} sessions, {} messages",
                    dir_path,
                    response.total_sessions_imported,
                    response.total_messages_imported
                );

                stats.total_files += response.total_files_processed;
                stats.successful_imports += response.successful_imports;
                stats.failed_imports += response.failed_imports;
                stats.total_sessions_imported += response.total_sessions_imported;
                stats.total_messages_imported += response.total_messages_imported;

                // Add directory-level result
                stats.results.push(ImportFileResult {
                    file_path: dir_path.clone(),
                    sessions_imported: response.total_sessions_imported,
                    messages_imported: response.total_messages_imported,
                    success: response.failed_imports == 0,
                    error: if response.errors.is_empty() {
                        None
                    } else {
                        Some(response.errors.join("; "))
                    },
                });
            }
            Err(e) => {
                log::error!("Failed to import from directory '{}': {}", dir_path, e);
                stats.failed_imports += 1;
                stats.results.push(ImportFileResult {
                    file_path: dir_path.clone(),
                    sessions_imported: 0,
                    messages_imported: 0,
                    success: false,
                    error: Some(e.to_string()),
                });
            }
        }
    }

    Ok(())
}
