use anyhow::{anyhow, Result};
use futures::future::join_all;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::sync::Arc;
use std::time::UNIX_EPOCH;
use tokio::sync::{mpsc, Semaphore};
use tokio::task::JoinHandle;
use tokio::time::Instant;

use crate::database::{
    ChatSessionRepository, DatabaseManager, MessageRepository, ProjectRepository,
};
use crate::parsers::ParserRegistry;

#[derive(Debug, Serialize, Deserialize)]
pub struct ScanRequest {
    pub directory_path: String,
    pub providers: Option<Vec<String>>,
    pub recursive: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChatFile {
    pub file_path: String,
    pub provider: String,
    pub estimated_sessions: i32,
    pub file_size_bytes: i64,
    pub last_modified: String,
    pub already_imported: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ScanResponse {
    pub files_found: Vec<ChatFile>,
    pub total_count: i32,
    pub scan_duration_ms: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ImportFileRequest {
    pub file_path: String,
    pub provider: Option<String>,
    pub project_name: Option<String>,
    pub overwrite_existing: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ImportFileResponse {
    pub sessions_imported: i32,
    pub messages_imported: i32,
    pub import_duration_ms: i32,
    pub file_size_bytes: i64,
    pub warnings: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BatchImportRequest {
    pub directory_path: String,
    pub providers: Option<Vec<String>>,
    pub project_name: Option<String>,
    pub overwrite_existing: Option<bool>,
    pub recursive: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BatchImportResponse {
    pub total_files_processed: i32,
    pub successful_imports: i32,
    pub failed_imports: i32,
    pub total_sessions_imported: i32,
    pub total_messages_imported: i32,
    pub batch_duration_ms: i32,
    pub errors: Vec<String>,
}

#[derive(Clone)]
pub struct ImportService {
    #[allow(dead_code)]
    db_manager: Arc<DatabaseManager>,
    max_concurrent_imports: usize,
}

impl ImportService {
    pub fn new(db_manager: Arc<DatabaseManager>) -> Self {
        Self {
            db_manager,
            max_concurrent_imports: 4, // Process up to 4 files concurrently
        }
    }

    pub fn with_concurrency(db_manager: Arc<DatabaseManager>, max_concurrent: usize) -> Self {
        Self {
            db_manager,
            max_concurrent_imports: max_concurrent,
        }
    }

    pub async fn scan_directory(&self, request: ScanRequest) -> Result<ScanResponse> {
        let start_time = Instant::now();

        let path = Path::new(&request.directory_path);
        if !path.exists() || !path.is_dir() {
            return Err(anyhow!(
                "Invalid directory path: {}",
                request.directory_path
            ));
        }

        let mut files_found = Vec::new();
        self.scan_directory_recursive(
            path,
            &request.providers,
            request.recursive.unwrap_or(false),
            &mut files_found,
        )?;

        let scan_duration_ms = start_time.elapsed().as_millis() as i32;

        Ok(ScanResponse {
            total_count: files_found.len() as i32,
            files_found,
            scan_duration_ms,
        })
    }

    fn scan_directory_recursive(
        &self,
        path: &Path,
        providers: &Option<Vec<String>>,
        recursive: bool,
        files_found: &mut Vec<ChatFile>,
    ) -> Result<()> {
        // Use ParserRegistry to scan for files
        let provider_filter = if let Some(ref providers) = providers {
            let llm_providers: Result<Vec<_>, _> = providers
                .iter()
                .map(|p| p.parse().map_err(|e| anyhow!("Invalid provider: {e}")))
                .collect();
            Some(llm_providers?)
        } else {
            None
        };

        let files = ParserRegistry::scan_directory(path, recursive, provider_filter.as_deref())?;

        for (file_path, provider) in files {
            let metadata = fs::metadata(&file_path)?;
            let file_size_bytes = metadata.len() as i64;
            let last_modified = metadata.modified()?.duration_since(UNIX_EPOCH)?.as_secs();
            let last_modified_str = format!("2024-01-01T{:02}:00:00Z", (last_modified % 24));

            files_found.push(ChatFile {
                file_path: file_path.to_string_lossy().to_string(),
                provider: provider.to_string(),
                estimated_sessions: self.estimate_sessions(file_size_bytes),
                file_size_bytes,
                last_modified: last_modified_str,
                already_imported: false, // TODO: Check database for existing imports
            });
        }

        Ok(())
    }

    fn detect_provider(&self, file_name: &str, extension: &str) -> String {
        if file_name.contains("claude") || extension == "jsonl" {
            "ClaudeCode".to_string()
        } else if file_name.contains("gemini") || extension == "json" {
            "Gemini".to_string() // Default JSON to Gemini
        } else {
            "Unknown".to_string()
        }
    }

    fn estimate_sessions(&self, file_size_bytes: i64) -> i32 {
        // Rough estimate: 1 session per 10KB
        std::cmp::max(1, (file_size_bytes / 10240) as i32)
    }

    pub async fn import_file(&self, request: ImportFileRequest) -> Result<ImportFileResponse> {
        let start_time = Instant::now();

        let path = Path::new(&request.file_path);
        if !path.exists() || !path.is_file() {
            return Err(anyhow!("Invalid file path: {}", request.file_path));
        }

        let metadata = fs::metadata(path)?;
        let file_size_bytes = metadata.len() as i64;

        let mut warnings = Vec::new();
        let mut sessions_imported = 0;
        let mut messages_imported = 0;

        // Detect provider if not provided (for validation)
        let _provider = request.provider.unwrap_or_else(|| {
            self.detect_provider(
                path.file_name().and_then(|n| n.to_str()).unwrap_or(""),
                path.extension().and_then(|e| e.to_str()).unwrap_or(""),
            )
        });

        // Parse the file using ParserRegistry
        let sessions = match ParserRegistry::parse_file(path).await {
            Ok(sessions) => sessions,
            Err(e) => {
                warnings.push(format!("Failed to parse file: {e}"));
                return Err(anyhow!("Failed to parse file: {e}"));
            }
        };

        if sessions.is_empty() {
            warnings.push("No sessions found in file".to_string());
            return Ok(ImportFileResponse {
                sessions_imported: 0,
                messages_imported: 0,
                import_duration_ms: start_time.elapsed().as_millis() as i32,
                file_size_bytes,
                warnings,
            });
        }

        // Import into database
        let session_repo = ChatSessionRepository::new(&self.db_manager);
        let message_repo = MessageRepository::new(&self.db_manager);
        let project_repo = ProjectRepository::new(&self.db_manager);

        for (session, messages) in sessions {
            // Check if session already exists
            let existing_session = session_repo.get_by_id(&session.id).await.ok().flatten();

            if existing_session.is_some() {
                if request.overwrite_existing.unwrap_or(false) {
                    // Delete existing session and its messages
                    if let Err(e) = message_repo.delete_by_session(&session.id).await {
                        warnings.push(format!(
                            "Failed to delete existing messages for session {}: {}",
                            session.id, e
                        ));
                        continue;
                    }

                    if let Err(e) = session_repo.delete(&session.id).await {
                        warnings.push(format!(
                            "Failed to delete existing session {}: {}",
                            session.id, e
                        ));
                        continue;
                    }

                    warnings.push(format!("Session {} overwritten", session.id));
                } else {
                    warnings.push(format!("Session {} already exists, skipping", session.id));
                    continue;
                }
            }

            // Create project if it doesn't exist
            if let Some(ref project_name) = session.project_name {
                if let Err(e) = project_repo.create_if_not_exists(project_name, None).await {
                    warnings.push(format!("Failed to create project {project_name}: {e}"));
                }
            }

            // Insert session
            if let Err(e) = session_repo.create(&session).await {
                warnings.push(format!("Failed to insert session {}: {}", session.id, e));
                continue;
            }

            sessions_imported += 1;

            // Insert messages
            for message in messages {
                if let Err(e) = message_repo.create(&message).await {
                    warnings.push(format!("Failed to insert message {}: {}", message.id, e));
                    continue;
                }
                messages_imported += 1;
            }
        }

        let import_duration_ms = start_time.elapsed().as_millis() as i32;

        Ok(ImportFileResponse {
            sessions_imported,
            messages_imported,
            import_duration_ms,
            file_size_bytes,
            warnings,
        })
    }

    pub async fn import_batch(&self, request: BatchImportRequest) -> Result<BatchImportResponse> {
        let start_time = Instant::now();

        // First scan for files
        let scan_request = ScanRequest {
            directory_path: request.directory_path,
            providers: request.providers,
            recursive: request.recursive,
        };

        let scan_response = self.scan_directory(scan_request).await?;

        if scan_response.files_found.is_empty() {
            return Ok(BatchImportResponse {
                total_files_processed: 0,
                successful_imports: 0,
                failed_imports: 0,
                total_sessions_imported: 0,
                total_messages_imported: 0,
                batch_duration_ms: start_time.elapsed().as_millis() as i32,
                errors: vec!["No files found for import".to_string()],
            });
        }

        // Create semaphore to limit concurrent imports
        let semaphore = Arc::new(Semaphore::new(self.max_concurrent_imports));

        // Create tasks for concurrent processing
        let mut tasks: Vec<JoinHandle<Result<ImportFileResponse>>> = Vec::new();

        for file in scan_response.files_found {
            let semaphore_clone = semaphore.clone();
            let import_request = ImportFileRequest {
                file_path: file.file_path.clone(),
                provider: Some(file.provider),
                project_name: request.project_name.clone(),
                overwrite_existing: request.overwrite_existing,
            };

            let service_clone = self.clone();
            let task = tokio::spawn(async move {
                let _permit = semaphore_clone.acquire().await.unwrap();
                service_clone.import_file(import_request).await
            });

            tasks.push(task);
        }

        // Wait for all tasks to complete
        let results = join_all(tasks).await;

        let mut successful_imports = 0;
        let mut failed_imports = 0;
        let mut total_sessions_imported = 0;
        let mut total_messages_imported = 0;
        let mut errors = Vec::new();

        for result in results {
            match result {
                Ok(Ok(import_response)) => {
                    successful_imports += 1;
                    total_sessions_imported += import_response.sessions_imported;
                    total_messages_imported += import_response.messages_imported;
                }
                Ok(Err(e)) => {
                    failed_imports += 1;
                    errors.push(e.to_string());
                }
                Err(e) => {
                    failed_imports += 1;
                    errors.push(format!("Task failed: {e}"));
                }
            }
        }

        let batch_duration_ms = start_time.elapsed().as_millis() as i32;

        Ok(BatchImportResponse {
            total_files_processed: scan_response.total_count,
            successful_imports,
            failed_imports,
            total_sessions_imported,
            total_messages_imported,
            batch_duration_ms,
            errors,
        })
    }

    /// Import files with progress reporting
    pub async fn import_batch_with_progress<F>(
        &self,
        request: BatchImportRequest,
        progress_callback: F,
    ) -> Result<BatchImportResponse>
    where
        F: Fn(i32, i32) + Send + Sync + 'static,
    {
        let start_time = Instant::now();

        // First scan for files
        let scan_request = ScanRequest {
            directory_path: request.directory_path,
            providers: request.providers,
            recursive: request.recursive,
        };

        let scan_response = self.scan_directory(scan_request).await?;
        let total_files = scan_response.files_found.len();

        if total_files == 0 {
            return Ok(BatchImportResponse {
                total_files_processed: 0,
                successful_imports: 0,
                failed_imports: 0,
                total_sessions_imported: 0,
                total_messages_imported: 0,
                batch_duration_ms: start_time.elapsed().as_millis() as i32,
                errors: vec!["No files found for import".to_string()],
            });
        }

        let (tx, mut rx) = mpsc::channel(100);
        let progress_callback = Arc::new(progress_callback);

        // Spawn progress reporter
        let progress_task = {
            let progress_callback = progress_callback.clone();
            tokio::spawn(async move {
                let mut completed = 0;
                while (rx.recv().await).is_some() {
                    completed += 1;
                    progress_callback(completed, total_files as i32);
                }
            })
        };

        // Create semaphore to limit concurrent imports
        let semaphore = Arc::new(Semaphore::new(self.max_concurrent_imports));
        let mut tasks: Vec<JoinHandle<Result<ImportFileResponse>>> = Vec::new();

        for file in scan_response.files_found {
            let semaphore_clone = semaphore.clone();
            let tx_clone = tx.clone();
            let import_request = ImportFileRequest {
                file_path: file.file_path.clone(),
                provider: Some(file.provider),
                project_name: request.project_name.clone(),
                overwrite_existing: request.overwrite_existing,
            };

            let service_clone = self.clone();
            let task = tokio::spawn(async move {
                let _permit = semaphore_clone.acquire().await.unwrap();
                let result = service_clone.import_file(import_request).await;
                let _ = tx_clone.send(()).await; // Report progress
                result
            });

            tasks.push(task);
        }

        // Drop the original sender so progress task can complete
        drop(tx);

        // Wait for all tasks to complete
        let results = join_all(tasks).await;

        // Wait for progress task to complete
        let _ = progress_task.await;

        let mut successful_imports = 0;
        let mut failed_imports = 0;
        let mut total_sessions_imported = 0;
        let mut total_messages_imported = 0;
        let mut errors = Vec::new();

        for result in results {
            match result {
                Ok(Ok(import_response)) => {
                    successful_imports += 1;
                    total_sessions_imported += import_response.sessions_imported;
                    total_messages_imported += import_response.messages_imported;
                }
                Ok(Err(e)) => {
                    failed_imports += 1;
                    errors.push(e.to_string());
                }
                Err(e) => {
                    failed_imports += 1;
                    errors.push(format!("Task failed: {e}"));
                }
            }
        }

        let batch_duration_ms = start_time.elapsed().as_millis() as i32;

        Ok(BatchImportResponse {
            total_files_processed: scan_response.total_count,
            successful_imports,
            failed_imports,
            total_sessions_imported,
            total_messages_imported,
            batch_duration_ms,
            errors,
        })
    }
}

// Note: ImportService requires a DatabaseManager, so no Default implementation
