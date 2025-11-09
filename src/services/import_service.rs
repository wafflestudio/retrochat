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
use uuid::Uuid;

use crate::database::{
    ChatSessionRepository, DatabaseManager, MessageRepository, ProjectRepository,
    ToolOperationRepository,
};
use crate::models::bash_metadata::BashMetadata;
use crate::models::ToolOperation;
use crate::parsers::ParserRegistry;
use crate::tools::parsers::{
    bash::BashParser, edit::EditParser, read::ReadParser, write::WriteParser, ToolData, ToolParser,
};
use crate::utils::bash_utils;

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
        // Use number of CPU cores, with a reasonable max
        let max_concurrent = num_cpus::get().clamp(4, 16);
        Self {
            db_manager,
            max_concurrent_imports: max_concurrent,
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
        if file_name.contains("claude") {
            "ClaudeCode".to_string()
        } else if file_name.contains("codex") {
            "Codex".to_string()
        } else if file_name.contains("cursor") {
            "CursorAgent".to_string()
        } else if file_name.contains("gemini") {
            "GeminiCLI".to_string()
        } else if extension == "jsonl" {
            "ClaudeCode".to_string() // Default JSONL to Claude
        } else if extension == "json" {
            "GeminiCLI".to_string() // Default JSON to Gemini
        } else if extension == "db" {
            "CursorAgent".to_string() // Default DB to Cursor
        } else {
            "Unknown".to_string()
        }
    }

    fn estimate_sessions(&self, file_size_bytes: i64) -> i32 {
        // Rough estimate: 1 session per 10KB
        std::cmp::max(1, (file_size_bytes / 10240) as i32)
    }

    /// Import sessions into the database
    ///
    /// Returns (sessions_imported, messages_imported, warnings)
    async fn import_sessions(
        &self,
        sessions: Vec<(crate::models::ChatSession, Vec<crate::models::Message>)>,
        overwrite_existing: bool,
    ) -> Result<(i32, i32, Vec<String>)> {
        let mut warnings = Vec::new();
        let mut sessions_imported = 0;
        let mut messages_imported = 0;

        let session_repo = ChatSessionRepository::new(&self.db_manager);
        let message_repo = MessageRepository::new(&self.db_manager);
        let project_repo = ProjectRepository::new(&self.db_manager);
        let tool_operation_repo = ToolOperationRepository::new(&self.db_manager);

        for (session, mut messages) in sessions {
            // Check if session already exists
            let existing_session = session_repo.get_by_id(&session.id).await.ok().flatten();

            if existing_session.is_some() {
                if overwrite_existing {
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

            // Extract and save tool operations FIRST (before messages)
            // Returns map of message_id -> (tool_operation_id, message_type)
            let tool_op_links = match self
                .extract_and_save_tool_operations(&tool_operation_repo, &messages)
                .await
            {
                Ok(links) => links,
                Err(e) => {
                    warnings.push(format!(
                        "Failed to create tool operations for session {}: {}",
                        session.id, e
                    ));
                    // Try to rollback session insertion
                    let _ = session_repo.delete(&session.id).await;
                    continue;
                }
            };

            // Update messages with tool_operation_id and message_type
            for message in &mut messages {
                if let Some((tool_op_id, msg_type)) = tool_op_links.get(&message.id) {
                    message.tool_operation_id = Some(*tool_op_id);
                    message.message_type = msg_type.clone();
                }
                // Clear transient fields before saving
                message.tool_uses = None;
                message.tool_results = None;
            }

            // Use bulk insert for messages
            let session_messages_imported = messages.len() as i32;
            if let Err(e) = message_repo.bulk_create(&messages).await {
                warnings.push(format!(
                    "Failed to bulk insert messages for session {}: {}",
                    session.id, e
                ));
                // Try to rollback session insertion
                let _ = session_repo.delete(&session.id).await;
                continue;
            }

            sessions_imported += 1;
            messages_imported += session_messages_imported;
        }

        Ok((sessions_imported, messages_imported, warnings))
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
                let error_msg = e.to_string();
                // Skip summary-only files silently (these are just metadata, not actual conversations)
                if error_msg.contains("only summary entries") {
                    return Ok(ImportFileResponse {
                        sessions_imported: 0,
                        messages_imported: 0,
                        import_duration_ms: start_time.elapsed().as_millis() as i32,
                        file_size_bytes,
                        warnings: vec![],
                    });
                }
                warnings.push(format!("Failed to parse file: {error_msg}"));
                return Err(anyhow!("Failed to parse file: {error_msg}"));
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

        // Import sessions into database
        let (sessions_imported, messages_imported, import_warnings) = self
            .import_sessions(sessions, request.overwrite_existing.unwrap_or(false))
            .await?;

        warnings.extend(import_warnings);

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
        let mut tasks: Vec<JoinHandle<(String, Result<ImportFileResponse>)>> = Vec::new();

        for file in scan_response.files_found {
            let semaphore_clone = semaphore.clone();
            let file_path = file.file_path.clone();
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
                (file_path, result)
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
                Ok((_file_path, Ok(import_response))) => {
                    successful_imports += 1;
                    total_sessions_imported += import_response.sessions_imported;
                    total_messages_imported += import_response.messages_imported;
                }
                Ok((file_path, Err(e))) => {
                    failed_imports += 1;
                    let error_msg = Self::format_import_error(&file_path, &e);
                    errors.push(error_msg);
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

    /// Extract tool operations from messages and save them to database
    /// Returns a map of message_id -> (tool_operation_id, message_type)
    async fn extract_and_save_tool_operations(
        &self,
        tool_operation_repo: &ToolOperationRepository,
        messages: &[crate::models::Message],
    ) -> Result<std::collections::HashMap<Uuid, (Uuid, crate::models::message::MessageType)>> {
        use crate::models::message::MessageType;
        let mut message_links = std::collections::HashMap::new();
        let mut tool_operations = Vec::new();

        // PHASE 1: Collect all tool_results from all messages
        // This allows matching tool_results that are in separate messages from tool_uses
        let mut global_tool_results: std::collections::HashMap<
            String,
            Vec<(&crate::models::message::ToolResult, Uuid)>,
        > = std::collections::HashMap::new();

        for message in messages {
            if let Some(tool_results) = &message.tool_results {
                for tool_result in tool_results {
                    global_tool_results
                        .entry(tool_result.tool_use_id.clone())
                        .or_default()
                        .push((tool_result, message.id));
                }
            }
        }

        // PHASE 2: Process tool_uses and create ToolOperations
        for message in messages {
            // Process tool_uses if present
            if let Some(tool_uses) = &message.tool_uses {
                for (idx, tool_use) in tool_uses.iter().enumerate() {
                    // Find matching tool_result from global map (could be in any message)
                    let tool_result_data = global_tool_results
                        .get(&tool_use.id)
                        .and_then(|vec| vec.first());

                    let tool_result = tool_result_data.map(|(tr, _)| *tr);
                    let tool_result_message_id = tool_result_data.map(|(_, mid)| *mid);

                    // Create base ToolOperation
                    let mut operation =
                        ToolOperation::from_tool_use(tool_use, tool_result, message.timestamp);

                    // Parse tool-specific data and extract metrics
                    match tool_use.name.as_str() {
                        "Edit" => {
                            let parser = EditParser;
                            if let Ok(parsed) = parser.parse(tool_use) {
                                if let ToolData::Edit(data) = parsed.data {
                                    operation = operation
                                        .with_file_path(data.file_path.clone())
                                        .with_file_type(data.is_code_file(), data.is_config_file())
                                        .with_line_metrics(data.lines_before(), data.lines_after())
                                        .with_edit_flags(
                                            data.is_bulk_replacement(),
                                            data.is_refactoring(),
                                        );
                                }
                            }
                        }
                        "Write" => {
                            let parser = WriteParser;
                            if let Ok(parsed) = parser.parse(tool_use) {
                                if let ToolData::Write(data) = parsed.data {
                                    operation = operation
                                        .with_file_path(data.file_path.clone())
                                        .with_file_type(data.is_code_file(), data.is_config_file())
                                        .with_line_metrics(None, data.lines_after());

                                    if let Some(size) = data.content_size {
                                        operation = operation.with_content_size(size as i32);
                                    }
                                }
                            }
                        }
                        "Read" => {
                            let parser = ReadParser;
                            if let Ok(parsed) = parser.parse(tool_use) {
                                if let ToolData::Read(data) = parsed.data {
                                    operation = operation
                                        .with_file_path(data.file_path.clone())
                                        .with_file_type(data.is_code_file(), data.is_config_file());
                                }
                            }
                        }
                        "Bash" => {
                            let parser = BashParser;
                            if let Ok(parsed) = parser.parse(tool_use) {
                                if let ToolData::Bash(data) = parsed.data {
                                    // Create bash metadata for the main operation
                                    let mut bash_metadata = BashMetadata::new(
                                        "BashCommand".to_string(),
                                        data.command.clone(),
                                    );

                                    // Extract stdout, stderr, and exit code from tool result
                                    if let Some(result) = tool_result {
                                        let (stdout, stderr) =
                                            bash_utils::extract_bash_output(result);
                                        let exit_code = bash_utils::extract_bash_exit_code(result);

                                        if let Some(stdout) = stdout {
                                            bash_metadata = bash_metadata.with_stdout(stdout);
                                        }
                                        if let Some(stderr) = stderr {
                                            bash_metadata = bash_metadata.with_stderr(stderr);
                                        }
                                        if let Some(exit_code) = exit_code {
                                            bash_metadata = bash_metadata.with_exit_code(exit_code);
                                        }
                                    }

                                    // Set the bash metadata on the main operation
                                    operation = operation.with_bash_metadata(bash_metadata);

                                    // For each file operation, create a separate ToolOperation
                                    if data.has_file_operations() {
                                        for file_op in &data.file_operations {
                                            for file_path in &file_op.file_paths {
                                                let mut file_operation =
                                                    ToolOperation::from_tool_use(
                                                        tool_use,
                                                        tool_result,
                                                        message.timestamp,
                                                    );

                                                // Create bash metadata for this specific file operation
                                                let mut file_bash_metadata = BashMetadata::new(
                                                    format!("{:?}", file_op.operation_type),
                                                    data.command.clone(),
                                                );

                                                // Extract stdout, stderr, and exit code from tool result
                                                if let Some(result) = tool_result {
                                                    let (stdout, stderr) =
                                                        bash_utils::extract_bash_output(result);
                                                    let exit_code =
                                                        bash_utils::extract_bash_exit_code(result);

                                                    if let Some(stdout) = stdout {
                                                        file_bash_metadata =
                                                            file_bash_metadata.with_stdout(stdout);
                                                    }
                                                    if let Some(stderr) = stderr {
                                                        file_bash_metadata =
                                                            file_bash_metadata.with_stderr(stderr);
                                                    }
                                                    if let Some(exit_code) = exit_code {
                                                        file_bash_metadata = file_bash_metadata
                                                            .with_exit_code(exit_code);
                                                    }
                                                }

                                                // Set file metadata and bash metadata
                                                file_operation = file_operation
                                                    .with_file_path(file_path.clone())
                                                    .with_bash_metadata(file_bash_metadata);

                                                // Determine file type based on extension
                                                let is_code = file_path.ends_with(".rs")
                                                    || file_path.ends_with(".js")
                                                    || file_path.ends_with(".ts")
                                                    || file_path.ends_with(".py")
                                                    || file_path.ends_with(".go")
                                                    || file_path.ends_with(".java");

                                                let is_config = file_path.ends_with("Cargo.toml")
                                                    || file_path.ends_with("package.json")
                                                    || file_path.ends_with(".yaml")
                                                    || file_path.ends_with(".yml")
                                                    || file_path.ends_with(".toml");

                                                file_operation = file_operation
                                                    .with_file_type(is_code, is_config);

                                                tool_operations.push(file_operation);
                                            }
                                        }
                                        // Mark that we've handled this operation
                                        operation = operation
                                            .with_file_path("__bash_handled__".to_string());
                                    }
                                }
                            }
                        }
                        _ => {
                            // For other tools (Task, etc.), just save the basic info
                            // File-related fields will be None
                        }
                    }

                    // Only add the original operation if it hasn't been handled by file operations
                    if !operation.is_file_operation()
                        || operation
                            .file_metadata
                            .as_ref()
                            .is_none_or(|meta| meta.file_path != "__bash_handled__")
                    {
                        tool_operations.push(operation.clone());
                    }

                    // Link the tool_use message (first tool_use only)
                    if idx == 0 {
                        message_links.insert(message.id, (operation.id, MessageType::ToolRequest));
                    }

                    // Link the tool_result message as well (if it's a different message)
                    if let Some(result_msg_id) = tool_result_message_id {
                        if result_msg_id != message.id {
                            message_links
                                .insert(result_msg_id, (operation.id, MessageType::ToolResult));
                        }
                    }
                }
            }
        }

        // Bulk create all tool operations
        if !tool_operations.is_empty() {
            tool_operation_repo.bulk_create(&tool_operations).await?;
        }

        Ok(message_links)
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
        let mut tasks: Vec<JoinHandle<(String, Result<ImportFileResponse>)>> = Vec::new();

        for file in scan_response.files_found {
            let semaphore_clone = semaphore.clone();
            let tx_clone = tx.clone();
            let file_path = file.file_path.clone();
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
                (file_path, result)
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
                Ok((_file_path, Ok(import_response))) => {
                    successful_imports += 1;
                    total_sessions_imported += import_response.sessions_imported;
                    total_messages_imported += import_response.messages_imported;
                }
                Ok((file_path, Err(e))) => {
                    failed_imports += 1;
                    let error_msg = Self::format_import_error(&file_path, &e);
                    errors.push(error_msg);
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

    /// Format import error with file path and truncate long messages
    fn format_import_error(file_path: &str, error: &anyhow::Error) -> String {
        let error_str = error.to_string();

        // Extract just the filename for cleaner display
        let file_name = std::path::Path::new(file_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(file_path);

        // Truncate error message if it contains long JSON content
        let max_error_length = 200;
        let error_summary = if error_str.len() > max_error_length {
            // Try to find a meaningful error message before JSON content
            if let Some(json_start) = error_str.find('{').or_else(|| error_str.find('[')) {
                if json_start < max_error_length {
                    format!(
                        "{}... (truncated JSON)",
                        &error_str[..json_start.min(max_error_length)]
                    )
                } else {
                    format!("{}...", &error_str[..max_error_length])
                }
            } else {
                format!("{}...", &error_str[..max_error_length])
            }
        } else {
            error_str.clone()
        };

        format!("[{}] {}", file_name, error_summary)
    }
}

// Note: ImportService requires a DatabaseManager, so no Default implementation

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::DatabaseManager;
    use crate::models::message::{Message, MessageRole, ToolResult, ToolUse};
    use chrono::Utc;
    use serde_json::json;

    #[tokio::test]
    async fn test_extract_tool_operations_with_separated_messages() {
        // Setup: Create in-memory database
        let db = DatabaseManager::open_in_memory().await.unwrap();
        let tool_operation_repo = ToolOperationRepository::new(&db);
        let service = ImportService::new(Arc::new(db));

        let session_id = Uuid::new_v4();
        let timestamp = Utc::now();

        // Create tool_use in message 1
        let tool_use = ToolUse {
            id: "test_tool_123".to_string(),
            name: "Read".to_string(),
            input: json!({"file_path": "/test/file.rs"}),
            raw: json!({}),
        };

        let msg1 = Message::new(
            session_id,
            MessageRole::Assistant,
            "Reading file".to_string(),
            timestamp,
            1,
        )
        .with_tool_uses(vec![tool_use]);

        // Create tool_result in message 2 (separate message)
        let tool_result = ToolResult {
            tool_use_id: "test_tool_123".to_string(),
            content: "File contents here".to_string(),
            is_error: false,
            details: None,
            raw: json!({}),
        };

        let msg2 = Message::new(
            session_id,
            MessageRole::Assistant,
            "File read successfully".to_string(),
            timestamp,
            2,
        )
        .with_tool_results(vec![tool_result]);

        let messages = vec![msg1.clone(), msg2.clone()];

        // Execute: Extract and save tool operations
        let result = service
            .extract_and_save_tool_operations(&tool_operation_repo, &messages)
            .await;

        assert!(
            result.is_ok(),
            "Should successfully extract tool operations"
        );
        let message_links = result.unwrap();

        // Verify: Both messages should be linked to the same ToolOperation
        assert_eq!(
            message_links.len(),
            2,
            "Both messages should be linked to tool operation"
        );

        let (tool_op_id_1, msg_type_1) = message_links.get(&msg1.id).unwrap();
        let (tool_op_id_2, msg_type_2) = message_links.get(&msg2.id).unwrap();

        // Both should reference the same ToolOperation
        assert_eq!(
            tool_op_id_1, tool_op_id_2,
            "Both messages should link to the same ToolOperation"
        );

        // Message types should be correct
        assert_eq!(
            *msg_type_1,
            crate::models::message::MessageType::ToolRequest
        );
        assert_eq!(*msg_type_2, crate::models::message::MessageType::ToolResult);

        // Verify ToolOperation was actually created in database
        let saved_op = tool_operation_repo.get_by_id(tool_op_id_1).await.unwrap();
        assert!(saved_op.is_some(), "ToolOperation should be saved");

        let op = saved_op.unwrap();
        assert_eq!(op.tool_name, "Read");
        assert_eq!(op.tool_use_id, "test_tool_123");
    }

    #[tokio::test]
    async fn test_extract_tool_operations_with_combined_message() {
        // Setup
        let db = DatabaseManager::open_in_memory().await.unwrap();
        let tool_operation_repo = ToolOperationRepository::new(&db);
        let service = ImportService::new(Arc::new(db));

        let session_id = Uuid::new_v4();
        let timestamp = Utc::now();

        // Create message with both tool_use and tool_result
        let tool_use = ToolUse {
            id: "test_tool_456".to_string(),
            name: "Write".to_string(),
            input: json!({"file_path": "/test/output.rs", "content": "fn main() {}"}),
            raw: json!({}),
        };

        let tool_result = ToolResult {
            tool_use_id: "test_tool_456".to_string(),
            content: "File written successfully".to_string(),
            is_error: false,
            details: None,
            raw: json!({}),
        };

        let msg = Message::new(
            session_id,
            MessageRole::Assistant,
            "Writing file".to_string(),
            timestamp,
            1,
        )
        .with_tool_uses(vec![tool_use])
        .with_tool_results(vec![tool_result]);

        let messages = vec![msg.clone()];

        // Execute
        let result = service
            .extract_and_save_tool_operations(&tool_operation_repo, &messages)
            .await;

        assert!(result.is_ok());
        let message_links = result.unwrap();

        // Verify: Only one message should be linked (same message contains both)
        assert_eq!(message_links.len(), 1);

        let (tool_op_id, msg_type) = message_links.get(&msg.id).unwrap();
        assert_eq!(*msg_type, crate::models::message::MessageType::ToolRequest);

        // Verify ToolOperation exists with result data
        let saved_op = tool_operation_repo.get_by_id(tool_op_id).await.unwrap();
        assert!(saved_op.is_some());

        let op = saved_op.unwrap();
        assert_eq!(op.tool_name, "Write");
        assert_eq!(op.success, Some(true)); // Should have result data
    }
}
