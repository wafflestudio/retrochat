use crate::OpenedFiles;
use std::path::PathBuf;
use tauri::{AppHandle, Emitter, Manager, State};

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
