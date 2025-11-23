// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod dto;

use commands::{
    analytics::{
        analyze_session, cancel_analysis, create_analysis, get_analysis_result,
        get_analysis_status, list_analyses, run_analysis,
    },
    file::{clear_opened_files, get_opened_files, handle_file_drop},
    session::{get_providers, get_session_detail, get_sessions, search_messages},
};
use retrochat::database::{config, DatabaseManager};
use retrochat::services::{
    google_ai::{GoogleAiClient, GoogleAiConfig},
    AnalyticsRequestService, QueryService,
};
use std::path::PathBuf;
use std::sync::{Arc, Mutex as StdMutex};
use tauri::Manager;
use tokio::sync::Mutex;

// Application state
pub struct AppState {
    pub db_manager: Arc<DatabaseManager>,
    pub query_service: Arc<QueryService>,
    pub analytics_service: Option<Arc<AnalyticsRequestService>>,
}

// State to store opened file paths
pub struct OpenedFiles(pub StdMutex<Vec<String>>);

#[tokio::main]
pub async fn run() -> anyhow::Result<()> {
    // Initialize database
    let db_path = config::get_default_db_path()?;
    let db_manager = Arc::new(DatabaseManager::new(&db_path).await?);

    // Initialize services
    let query_service = Arc::new(QueryService::with_database(db_manager.clone()));

    // Initialize analytics service if Google AI API key is available
    let analytics_service = match std::env::var(retrochat::env::apis::GOOGLE_AI_API_KEY) {
        Ok(api_key) if !api_key.is_empty() => {
            let google_ai_config = GoogleAiConfig::new(api_key);
            match GoogleAiClient::new(google_ai_config) {
                Ok(client) => Some(Arc::new(AnalyticsRequestService::new(
                    db_manager.clone(),
                    client,
                ))),
                Err(e) => {
                    eprintln!("Warning: Failed to initialize Google AI client: {}", e);
                    None
                }
            }
        }
        _ => None,
    };

    let app_state = Arc::new(Mutex::new(AppState {
        db_manager,
        query_service,
        analytics_service,
    }));

    tauri::Builder::default()
        .plugin(tauri_plugin_fs::init())
        .setup(|app| {
            #[cfg(debug_assertions)]
            {
                let window = app.get_webview_window("main").unwrap();
                window.open_devtools();
                window.close_devtools();
            }

            // Handle file associations on Windows/Linux (command-line arguments)
            #[cfg(not(any(target_os = "macos", target_os = "ios")))]
            {
                let args: Vec<String> = std::env::args().collect();
                if args.len() > 1 {
                    let file_paths: Vec<PathBuf> = args[1..]
                        .iter()
                        .filter(|arg| {
                            let path = PathBuf::from(arg);
                            path.extension()
                                .and_then(|ext| ext.to_str())
                                .map(|ext| ext == "json" || ext == "jsonl")
                                .unwrap_or(false)
                        })
                        .map(PathBuf::from)
                        .collect();

                    if !file_paths.is_empty() {
                        handle_file_drop(app.handle().clone(), file_paths);
                    }
                }
            }

            Ok(())
        })
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(app_state)
        .manage(OpenedFiles(StdMutex::new(Vec::new())))
        .invoke_handler(tauri::generate_handler![
            get_sessions,
            get_session_detail,
            search_messages,
            get_providers,
            analyze_session,
            create_analysis,
            run_analysis,
            get_analysis_status,
            get_analysis_result,
            list_analyses,
            cancel_analysis,
            get_opened_files,
            clear_opened_files,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app, event| {
            match event {
                // Handle file associations on macOS/iOS (file open events)
                #[cfg(any(target_os = "macos", target_os = "ios"))]
                tauri::RunEvent::Opened { urls } => {
                    let file_paths: Vec<PathBuf> = urls
                        .iter()
                        .filter_map(|url| {
                            url.to_file_path().ok().and_then(|path| {
                                path.extension()
                                    .and_then(|ext| ext.to_str())
                                    .map(|ext| ext == "json" || ext == "jsonl")
                                    .and_then(|valid| if valid { Some(path) } else { None })
                            })
                        })
                        .collect();

                    if !file_paths.is_empty() {
                        handle_file_drop(app.clone(), file_paths);
                    }
                }
                // Handle drag-and-drop events
                tauri::RunEvent::WindowEvent {
                    label: _,
                    event: tauri::WindowEvent::DragDrop(tauri::DragDropEvent::Drop { paths, .. }),
                    ..
                } => {
                    let file_paths: Vec<PathBuf> = paths
                        .into_iter()
                        .filter(|path| {
                            path.extension()
                                .and_then(|ext| ext.to_str())
                                .map(|ext| ext == "json" || ext == "jsonl")
                                .unwrap_or(false)
                        })
                        .collect();

                    if !file_paths.is_empty() {
                        handle_file_drop(app.clone(), file_paths);
                    }
                }
                _ => {}
            }
        });

    Ok(())
}
