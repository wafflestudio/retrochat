use anyhow::{Context, Result as AnyhowResult};
use chrono::{DateTime, Utc};
use sqlx::{sqlite::SqliteRow, Pool, Row, Sqlite};
use uuid::Uuid;

use super::connection::DatabaseManager;
use crate::models::ToolOperation;

pub struct ToolOperationRepository {
    pool: Pool<Sqlite>,
}

impl ToolOperationRepository {
    pub fn new(db: &DatabaseManager) -> Self {
        Self {
            pool: db.pool().clone(),
        }
    }

    pub async fn create(&self, operation: &ToolOperation) -> AnyhowResult<()> {
        let raw_input_json = operation
            .raw_input
            .as_ref()
            .and_then(|v| serde_json::to_string(v).ok());

        let raw_result_json = operation
            .raw_result
            .as_ref()
            .and_then(|v| serde_json::to_string(v).ok());

        sqlx::query(
            r#"
            INSERT INTO tool_operations (
                id, message_id, tool_use_id, session_id, tool_name, timestamp,
                file_path, file_extension, is_code_file, is_config_file,
                lines_before, lines_after, lines_added, lines_removed, content_size,
                is_bulk_edit, is_refactoring,
                success, result_summary, raw_input, raw_result, created_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(operation.id.to_string())
        .bind(operation.message_id.to_string())
        .bind(&operation.tool_use_id)
        .bind(operation.session_id.to_string())
        .bind(&operation.tool_name)
        .bind(operation.timestamp.to_rfc3339())
        .bind(&operation.file_path)
        .bind(&operation.file_extension)
        .bind(operation.is_code_file)
        .bind(operation.is_config_file)
        .bind(operation.lines_before)
        .bind(operation.lines_after)
        .bind(operation.lines_added)
        .bind(operation.lines_removed)
        .bind(operation.content_size)
        .bind(operation.is_bulk_edit)
        .bind(operation.is_refactoring)
        .bind(operation.success)
        .bind(&operation.result_summary)
        .bind(raw_input_json)
        .bind(raw_result_json)
        .bind(operation.created_at.to_rfc3339())
        .execute(&self.pool)
        .await
        .context("Failed to create tool operation")?;

        Ok(())
    }

    pub async fn bulk_create(&self, operations: &[ToolOperation]) -> AnyhowResult<()> {
        if operations.is_empty() {
            return Ok(());
        }

        let mut tx = self.pool.begin().await?;

        for operation in operations {
            let raw_input_json = operation
                .raw_input
                .as_ref()
                .and_then(|v| serde_json::to_string(v).ok());

            let raw_result_json = operation
                .raw_result
                .as_ref()
                .and_then(|v| serde_json::to_string(v).ok());

            sqlx::query(
                r#"
                INSERT INTO tool_operations (
                    id, message_id, tool_use_id, session_id, tool_name, timestamp,
                    file_path, file_extension, is_code_file, is_config_file,
                    lines_before, lines_after, lines_added, lines_removed, content_size,
                    is_bulk_edit, is_refactoring,
                    success, result_summary, raw_input, raw_result, created_at
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                "#,
            )
            .bind(operation.id.to_string())
            .bind(operation.message_id.to_string())
            .bind(&operation.tool_use_id)
            .bind(operation.session_id.to_string())
            .bind(&operation.tool_name)
            .bind(operation.timestamp.to_rfc3339())
            .bind(&operation.file_path)
            .bind(&operation.file_extension)
            .bind(operation.is_code_file)
            .bind(operation.is_config_file)
            .bind(operation.lines_before)
            .bind(operation.lines_after)
            .bind(operation.lines_added)
            .bind(operation.lines_removed)
            .bind(operation.content_size)
            .bind(operation.is_bulk_edit)
            .bind(operation.is_refactoring)
            .bind(operation.success)
            .bind(&operation.result_summary)
            .bind(raw_input_json)
            .bind(raw_result_json)
            .bind(operation.created_at.to_rfc3339())
            .execute(&mut *tx)
            .await
            .context("Failed to create tool operation in bulk")?;
        }

        tx.commit().await?;
        Ok(())
    }

    pub async fn get_by_id(&self, id: &Uuid) -> AnyhowResult<Option<ToolOperation>> {
        let row = sqlx::query(
            r#"
            SELECT id, message_id, tool_use_id, session_id, tool_name, timestamp,
                   file_path, file_extension, is_code_file, is_config_file,
                   lines_before, lines_after, lines_added, lines_removed, content_size,
                   is_bulk_edit, is_refactoring,
                   success, result_summary, raw_input, raw_result, created_at
            FROM tool_operations
            WHERE id = ?
            "#,
        )
        .bind(id.to_string())
        .fetch_optional(&self.pool)
        .await
        .context("Failed to fetch tool operation by ID")?;

        match row {
            Some(row) => {
                let operation = self.row_to_tool_operation(&row)?;
                Ok(Some(operation))
            }
            None => Ok(None),
        }
    }

    pub async fn get_by_session(&self, session_id: &Uuid) -> AnyhowResult<Vec<ToolOperation>> {
        let rows = sqlx::query(
            r#"
            SELECT id, message_id, tool_use_id, session_id, tool_name, timestamp,
                   file_path, file_extension, is_code_file, is_config_file,
                   lines_before, lines_after, lines_added, lines_removed, content_size,
                   is_bulk_edit, is_refactoring,
                   success, result_summary, raw_input, raw_result, created_at
            FROM tool_operations
            WHERE session_id = ?
            ORDER BY timestamp ASC
            "#,
        )
        .bind(session_id.to_string())
        .fetch_all(&self.pool)
        .await
        .context("Failed to fetch tool operations by session")?;

        let mut operations = Vec::new();
        for row in rows {
            let operation = self.row_to_tool_operation(&row)?;
            operations.push(operation);
        }

        Ok(operations)
    }

    pub async fn get_by_message(&self, message_id: &Uuid) -> AnyhowResult<Vec<ToolOperation>> {
        let rows = sqlx::query(
            r#"
            SELECT id, message_id, tool_use_id, session_id, tool_name, timestamp,
                   file_path, file_extension, is_code_file, is_config_file,
                   lines_before, lines_after, lines_added, lines_removed, content_size,
                   is_bulk_edit, is_refactoring,
                   success, result_summary, raw_input, raw_result, created_at
            FROM tool_operations
            WHERE message_id = ?
            ORDER BY timestamp ASC
            "#,
        )
        .bind(message_id.to_string())
        .fetch_all(&self.pool)
        .await
        .context("Failed to fetch tool operations by message")?;

        let mut operations = Vec::new();
        for row in rows {
            let operation = self.row_to_tool_operation(&row)?;
            operations.push(operation);
        }

        Ok(operations)
    }

    /// Get only file operations (operations with file_path)
    pub async fn get_file_operations(&self, session_id: &Uuid) -> AnyhowResult<Vec<ToolOperation>> {
        let rows = sqlx::query(
            r#"
            SELECT id, message_id, tool_use_id, session_id, tool_name, timestamp,
                   file_path, file_extension, is_code_file, is_config_file,
                   lines_before, lines_after, lines_added, lines_removed, content_size,
                   is_bulk_edit, is_refactoring,
                   success, result_summary, raw_input, raw_result, created_at
            FROM tool_operations
            WHERE session_id = ? AND file_path IS NOT NULL
            ORDER BY timestamp ASC
            "#,
        )
        .bind(session_id.to_string())
        .fetch_all(&self.pool)
        .await
        .context("Failed to fetch file operations")?;

        let mut operations = Vec::new();
        for row in rows {
            let operation = self.row_to_tool_operation(&row)?;
            operations.push(operation);
        }

        Ok(operations)
    }

    /// Get file history for a specific file path
    pub async fn get_file_history(&self, file_path: &str) -> AnyhowResult<Vec<ToolOperation>> {
        let rows = sqlx::query(
            r#"
            SELECT id, message_id, tool_use_id, session_id, tool_name, timestamp,
                   file_path, file_extension, is_code_file, is_config_file,
                   lines_before, lines_after, lines_added, lines_removed, content_size,
                   is_bulk_edit, is_refactoring,
                   success, result_summary, raw_input, raw_result, created_at
            FROM tool_operations
            WHERE file_path = ?
            ORDER BY timestamp ASC
            "#,
        )
        .bind(file_path)
        .fetch_all(&self.pool)
        .await
        .context("Failed to fetch file history")?;

        let mut operations = Vec::new();
        for row in rows {
            let operation = self.row_to_tool_operation(&row)?;
            operations.push(operation);
        }

        Ok(operations)
    }

    /// Get tool usage statistics for a session
    pub async fn get_tool_usage_stats(
        &self,
        session_id: &Uuid,
    ) -> AnyhowResult<Vec<(String, i64)>> {
        let rows = sqlx::query(
            r#"
            SELECT tool_name, COUNT(*) as count
            FROM tool_operations
            WHERE session_id = ?
            GROUP BY tool_name
            ORDER BY count DESC
            "#,
        )
        .bind(session_id.to_string())
        .fetch_all(&self.pool)
        .await
        .context("Failed to fetch tool usage statistics")?;

        let mut stats = Vec::new();
        for row in rows {
            let tool_name: String = row.try_get("tool_name")?;
            let count: i64 = row.try_get("count")?;
            stats.push((tool_name, count));
        }

        Ok(stats)
    }

    /// Get file type statistics (code vs config vs other)
    pub async fn get_file_type_stats(&self, session_id: &Uuid) -> AnyhowResult<(i64, i64, i64)> {
        let row = sqlx::query(
            r#"
            SELECT
                SUM(CASE WHEN is_code_file = 1 THEN 1 ELSE 0 END) as code_files,
                SUM(CASE WHEN is_config_file = 1 THEN 1 ELSE 0 END) as config_files,
                SUM(CASE WHEN is_code_file = 0 AND is_config_file = 0 AND file_path IS NOT NULL THEN 1 ELSE 0 END) as other_files
            FROM tool_operations
            WHERE session_id = ? AND file_path IS NOT NULL
            "#,
        )
        .bind(session_id.to_string())
        .fetch_one(&self.pool)
        .await
        .context("Failed to fetch file type statistics")?;

        let code_files: i64 = row.try_get("code_files").unwrap_or(0);
        let config_files: i64 = row.try_get("config_files").unwrap_or(0);
        let other_files: i64 = row.try_get("other_files").unwrap_or(0);

        Ok((code_files, config_files, other_files))
    }

    /// Get total line changes for a session
    pub async fn get_total_line_changes(&self, session_id: &Uuid) -> AnyhowResult<(i64, i64)> {
        let row = sqlx::query(
            r#"
            SELECT
                COALESCE(SUM(lines_added), 0) as total_added,
                COALESCE(SUM(lines_removed), 0) as total_removed
            FROM tool_operations
            WHERE session_id = ?
            "#,
        )
        .bind(session_id.to_string())
        .fetch_one(&self.pool)
        .await
        .context("Failed to fetch total line changes")?;

        let total_added: i64 = row.try_get("total_added")?;
        let total_removed: i64 = row.try_get("total_removed")?;

        Ok((total_added, total_removed))
    }

    /// Get most modified files for a session
    pub async fn get_most_modified_files(
        &self,
        session_id: &Uuid,
        limit: i64,
    ) -> AnyhowResult<Vec<(String, i64, i64, i64)>> {
        let rows = sqlx::query(
            r#"
            SELECT
                file_path,
                COUNT(*) as modification_count,
                COALESCE(SUM(lines_added), 0) as total_lines_added,
                COALESCE(SUM(lines_removed), 0) as total_lines_removed
            FROM tool_operations
            WHERE session_id = ?
              AND file_path IS NOT NULL
              AND tool_name IN ('Write', 'Edit')
            GROUP BY file_path
            ORDER BY modification_count DESC
            LIMIT ?
            "#,
        )
        .bind(session_id.to_string())
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .context("Failed to fetch most modified files")?;

        let mut results = Vec::new();
        for row in rows {
            let file_path: String = row.try_get("file_path")?;
            let modification_count: i64 = row.try_get("modification_count")?;
            let total_lines_added: i64 = row.try_get("total_lines_added")?;
            let total_lines_removed: i64 = row.try_get("total_lines_removed")?;
            results.push((
                file_path,
                modification_count,
                total_lines_added,
                total_lines_removed,
            ));
        }

        Ok(results)
    }

    pub async fn delete_by_session(&self, session_id: &Uuid) -> AnyhowResult<u64> {
        let result = sqlx::query("DELETE FROM tool_operations WHERE session_id = ?")
            .bind(session_id.to_string())
            .execute(&self.pool)
            .await
            .context("Failed to delete tool operations by session")?;

        Ok(result.rows_affected())
    }

    pub async fn count_by_session(&self, session_id: &Uuid) -> AnyhowResult<i64> {
        let count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM tool_operations WHERE session_id = ?")
                .bind(session_id.to_string())
                .fetch_one(&self.pool)
                .await
                .context("Failed to count tool operations by session")?;

        Ok(count)
    }

    fn row_to_tool_operation(&self, row: &SqliteRow) -> AnyhowResult<ToolOperation> {
        let id_str: String = row.try_get("id")?;
        let message_id_str: String = row.try_get("message_id")?;
        let tool_use_id: String = row.try_get("tool_use_id")?;
        let session_id_str: String = row.try_get("session_id")?;
        let tool_name: String = row.try_get("tool_name")?;
        let timestamp_str: String = row.try_get("timestamp")?;

        let id = Uuid::parse_str(&id_str).context("Invalid tool operation ID format")?;
        let message_id = Uuid::parse_str(&message_id_str).context("Invalid message ID format")?;
        let session_id = Uuid::parse_str(&session_id_str).context("Invalid session ID format")?;
        let timestamp = DateTime::parse_from_rfc3339(&timestamp_str)
            .context("Invalid timestamp format")?
            .with_timezone(&Utc);

        let file_path: Option<String> = row.try_get("file_path").ok();
        let file_extension: Option<String> = row.try_get("file_extension").ok();
        let is_code_file: Option<bool> = row.try_get("is_code_file").ok();
        let is_config_file: Option<bool> = row.try_get("is_config_file").ok();

        let lines_before: Option<i32> = row.try_get("lines_before").ok();
        let lines_after: Option<i32> = row.try_get("lines_after").ok();
        let lines_added: Option<i32> = row.try_get("lines_added").ok();
        let lines_removed: Option<i32> = row.try_get("lines_removed").ok();
        let content_size: Option<i32> = row.try_get("content_size").ok();

        let is_bulk_edit: Option<bool> = row.try_get("is_bulk_edit").ok();
        let is_refactoring: Option<bool> = row.try_get("is_refactoring").ok();

        let success: Option<bool> = row.try_get("success").ok();
        let result_summary: Option<String> = row.try_get("result_summary").ok();

        let raw_input_json: Option<String> = row.try_get("raw_input").ok();
        let raw_result_json: Option<String> = row.try_get("raw_result").ok();

        let raw_input = raw_input_json.and_then(|json| serde_json::from_str(&json).ok());
        let raw_result = raw_result_json.and_then(|json| serde_json::from_str(&json).ok());

        let created_at_str: String = row.try_get("created_at")?;
        let created_at = DateTime::parse_from_rfc3339(&created_at_str)
            .context("Invalid created_at format")?
            .with_timezone(&Utc);

        Ok(ToolOperation {
            id,
            message_id,
            tool_use_id,
            session_id,
            tool_name,
            timestamp,
            file_path,
            file_extension,
            is_code_file,
            is_config_file,
            lines_before,
            lines_after,
            lines_added,
            lines_removed,
            content_size,
            is_bulk_edit,
            is_refactoring,
            success,
            result_summary,
            raw_input,
            raw_result,
            created_at,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::DatabaseManager;

    #[tokio::test]
    async fn test_create_and_get_tool_operation() {
        use crate::database::{ChatSessionRepository, MessageRepository};
        use crate::models::{ChatSession, Message, MessageRole, Provider, SessionState};

        let db = DatabaseManager::open_in_memory().await.unwrap();
        let repo = ToolOperationRepository::new(&db);
        let session_repo = ChatSessionRepository::new(&db);
        let message_repo = MessageRepository::new(&db);

        // Create session first
        let session_id = Uuid::new_v4();
        let mut session = ChatSession::new(
            Provider::ClaudeCode,
            "/test/file.jsonl".to_string(),
            "test_hash".to_string(),
            Utc::now(),
        );
        session.id = session_id;
        session.set_state(SessionState::Imported);
        session_repo.create(&session).await.unwrap();

        // Create message
        let message_id = Uuid::new_v4();
        let mut message = Message::new(
            session_id,
            MessageRole::Assistant,
            "test message".to_string(),
            Utc::now(),
            1,
        );
        message.id = message_id;
        message_repo.create(&message).await.unwrap();

        // Now create tool operation
        let operation = ToolOperation::new(
            message_id,
            "test_tool_use_id".to_string(),
            session_id,
            "Write".to_string(),
            Utc::now(),
        )
        .with_file_path("/test/file.rs".to_string())
        .with_file_type(true, false)
        .with_line_metrics(None, Some(10))
        .with_success(true);

        repo.create(&operation).await.unwrap();

        let retrieved = repo.get_by_id(&operation.id).await.unwrap();
        assert!(retrieved.is_some());

        let retrieved_op = retrieved.unwrap();
        assert_eq!(retrieved_op.tool_name, "Write");
        assert_eq!(retrieved_op.file_path, Some("/test/file.rs".to_string()));
        assert_eq!(retrieved_op.is_code_file, Some(true));
    }

    #[tokio::test]
    async fn test_get_by_session() {
        use crate::database::{ChatSessionRepository, MessageRepository};
        use crate::models::{ChatSession, Message, MessageRole, Provider, SessionState};

        let db = DatabaseManager::open_in_memory().await.unwrap();
        let repo = ToolOperationRepository::new(&db);
        let session_repo = ChatSessionRepository::new(&db);
        let message_repo = MessageRepository::new(&db);

        // Create session first
        let session_id = Uuid::new_v4();
        let mut session = ChatSession::new(
            Provider::ClaudeCode,
            "/test/file.jsonl".to_string(),
            "test_hash".to_string(),
            Utc::now(),
        );
        session.id = session_id;
        session.set_state(SessionState::Imported);
        session_repo.create(&session).await.unwrap();

        // Create message
        let message_id = Uuid::new_v4();
        let mut message = Message::new(
            session_id,
            MessageRole::Assistant,
            "test message".to_string(),
            Utc::now(),
            1,
        );
        message.id = message_id;
        message_repo.create(&message).await.unwrap();

        // Create multiple operations
        for i in 0..3 {
            let operation = ToolOperation::new(
                message_id,
                format!("tool_use_{i}"),
                session_id,
                "Edit".to_string(),
                Utc::now(),
            );
            repo.create(&operation).await.unwrap();
        }

        let operations = repo.get_by_session(&session_id).await.unwrap();
        assert_eq!(operations.len(), 3);
    }
}
