use crate::dto::{ImportFileResult, ImportSessionsResponse};
use crate::{AppState, OpenedFiles};
use retrochat::services::ImportFileRequest;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager, State};
use tokio::sync::Mutex;

// Handler for file associations and drops
pub fn handle_file_drop(app: AppHandle, files: Vec<PathBuf>) {
    if files.is_empty() {
        return;
    }

    let file_paths: Vec<String> = files
        .into_iter()
        .filter_map(|path| path.to_str().map(|s| s.to_string()))
        .collect();

    if let Some(opened_files) = app.try_state::<OpenedFiles>() {
        if let Ok(mut files) = opened_files.0.lock() {
            *files = file_paths.clone();
        }
    }

    // Emit event to frontend with the opened files
    let _ = app.emit("file-opened", file_paths);
}

// Command to get opened files
#[tauri::command]
pub fn get_opened_files(state: State<OpenedFiles>) -> Vec<String> {
    state.0.lock().unwrap().clone()
}

// Command to clear opened files
#[tauri::command]
pub fn clear_opened_files(state: State<OpenedFiles>) {
    state.0.lock().unwrap().clear()
}

// Command to import sessions from files
#[tauri::command]
pub async fn import_sessions(
    state: State<'_, Arc<Mutex<AppState>>>,
    file_paths: Vec<String>,
) -> Result<ImportSessionsResponse, String> {
    let state_guard = state.lock().await;
    let import_service = &state_guard.import_service;

    let mut results = Vec::new();
    let mut total_sessions_imported = 0;
    let mut total_messages_imported = 0;
    let mut successful_imports = 0;
    let mut failed_imports = 0;

    // Import each file
    for file_path in &file_paths {
        let request = ImportFileRequest {
            file_path: file_path.clone(),
            provider: None, // Auto-detect via retrochat lib
            project_name: None,
            overwrite_existing: Some(false),
        };

        match import_service.import_file(request).await {
            Ok(response) => {
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

    Ok(ImportSessionsResponse {
        total_files: file_paths.len() as i32,
        successful_imports,
        failed_imports,
        total_sessions_imported,
        total_messages_imported,
        results,
    })
}
