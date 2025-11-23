use crate::dto::{ImportFileResult, ImportSessionsResponse};
use crate::{AppState, OpenedFiles};
use retrochat::services::ImportFileRequest;
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
