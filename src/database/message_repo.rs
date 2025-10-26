use anyhow::{Context, Result as AnyhowResult};
use chrono::{DateTime, Utc};
use sqlx::{sqlite::SqliteRow, Pool, Row, Sqlite};
use std::str::FromStr;
use uuid::Uuid;

use super::connection::DatabaseManager;
use crate::models::message::{Message, MessageRole};

pub struct MessageRepository {
    pool: Pool<Sqlite>,
}

impl MessageRepository {
    pub fn new(db: &DatabaseManager) -> Self {
        Self {
            pool: db.pool().clone(),
        }
    }

    pub async fn create(&self, message: &Message) -> AnyhowResult<()> {
        sqlx::query(
            r#"
            INSERT INTO messages (
                id, session_id, role, content, timestamp, token_count,
                metadata, sequence_number, message_type, tool_operation_id
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(message.id.to_string())
        .bind(message.session_id.to_string())
        .bind(message.role.to_string())
        .bind(&message.content)
        .bind(message.timestamp.to_rfc3339())
        .bind(message.token_count)
        .bind("{}") // metadata
        .bind(message.sequence_number)
        .bind(message.message_type.to_string())
        .bind(message.tool_operation_id.map(|id| id.to_string()))
        .execute(&self.pool)
        .await
        .context("Failed to create message")?;

        Ok(())
    }

    pub async fn get_by_id(&self, id: &Uuid) -> AnyhowResult<Option<Message>> {
        let row = sqlx::query(
            r#"
            SELECT id, session_id, role, content, timestamp, token_count,
                   metadata, sequence_number, message_type, tool_operation_id
            FROM messages
            WHERE id = ?
            "#,
        )
        .bind(id.to_string())
        .fetch_optional(&self.pool)
        .await
        .context("Failed to fetch message by ID")?;

        match row {
            Some(row) => {
                let message = self.row_to_message(&row)?;
                Ok(Some(message))
            }
            None => Ok(None),
        }
    }

    pub async fn get_by_session_id(&self, session_id: &Uuid) -> AnyhowResult<Vec<Message>> {
        let rows = sqlx::query(
            r#"
            SELECT id, session_id, role, content, timestamp, token_count,
                   metadata, sequence_number, message_type, tool_operation_id
            FROM messages
            WHERE session_id = ?
            ORDER BY sequence_number ASC
            "#,
        )
        .bind(session_id.to_string())
        .fetch_all(&self.pool)
        .await
        .context("Failed to fetch messages by session ID")?;

        let mut messages = Vec::new();
        for row in rows {
            let message = self.row_to_message(&row)?;
            messages.push(message);
        }

        Ok(messages)
    }

    // Alias for backward compatibility
    pub async fn get_by_session(&self, session_id: &Uuid) -> AnyhowResult<Vec<Message>> {
        self.get_by_session_id(session_id).await
    }

    pub async fn search_content(
        &self,
        query: &str,
        limit: Option<i64>,
    ) -> AnyhowResult<Vec<Message>> {
        let limit = limit.unwrap_or(100);

        let rows = sqlx::query(
            r#"
            SELECT m.id, m.session_id, m.role, m.content, m.timestamp,
                   m.token_count, m.metadata, m.sequence_number,
                   m.message_type, m.tool_operation_id
            FROM messages m
            JOIN messages_fts fts ON m.rowid = fts.rowid
            WHERE messages_fts MATCH ?
            ORDER BY fts.rank
            LIMIT ?
            "#,
        )
        .bind(query)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .context("Failed to search messages")?;

        let mut messages = Vec::new();
        for row in rows {
            let message = self.row_to_message(&row)?;
            messages.push(message);
        }

        Ok(messages)
    }

    pub async fn search_content_with_filters(
        &self,
        query: &str,
        session_id: Option<&Uuid>,
        role: Option<&str>,
        limit: Option<i64>,
    ) -> AnyhowResult<Vec<Message>> {
        let limit = limit.unwrap_or(100);

        let mut sql = r#"
            SELECT m.id, m.session_id, m.role, m.content, m.timestamp,
                   m.token_count, m.metadata, m.sequence_number,
                   m.message_type, m.tool_operation_id
            FROM messages m
            JOIN messages_fts fts ON m.rowid = fts.rowid
            WHERE messages_fts MATCH ?
        "#
        .to_string();

        let mut params = vec![query.to_string()];

        if let Some(session_id) = session_id {
            sql.push_str(" AND m.session_id = ?");
            params.push(session_id.to_string());
        }

        if let Some(role) = role {
            sql.push_str(" AND m.role = ?");
            params.push(role.to_string());
        }

        sql.push_str(" ORDER BY fts.rank LIMIT ?");
        params.push(limit.to_string());

        let mut query_builder = sqlx::query(&sql);
        for param in &params {
            query_builder = query_builder.bind(param);
        }

        let rows = query_builder
            .fetch_all(&self.pool)
            .await
            .context("Failed to search messages with filters")?;

        let mut messages = Vec::new();
        for row in rows {
            let message = self.row_to_message(&row)?;
            messages.push(message);
        }

        Ok(messages)
    }

    pub async fn search_content_with_time_filters(
        &self,
        query: &str,
        session_id: Option<&Uuid>,
        role: Option<&str>,
        from: Option<DateTime<Utc>>,
        to: Option<DateTime<Utc>>,
        limit: Option<i64>,
    ) -> AnyhowResult<Vec<Message>> {
        let limit = limit.unwrap_or(100);

        let mut sql = r#"
            SELECT m.id, m.session_id, m.role, m.content, m.timestamp,
                   m.token_count, m.metadata, m.sequence_number,
                   m.message_type, m.tool_operation_id
            FROM messages m
            JOIN messages_fts fts ON m.rowid = fts.rowid
            WHERE messages_fts MATCH ?
        "#
        .to_string();

        let mut params = vec![query.to_string()];

        if let Some(session_id) = session_id {
            sql.push_str(" AND m.session_id = ?");
            params.push(session_id.to_string());
        }

        if let Some(role) = role {
            sql.push_str(" AND m.role = ?");
            params.push(role.to_string());
        }

        if let Some(from_time) = from {
            sql.push_str(" AND m.timestamp >= ?");
            params.push(from_time.to_rfc3339());
        }

        if let Some(to_time) = to {
            sql.push_str(" AND m.timestamp <= ?");
            params.push(to_time.to_rfc3339());
        }

        sql.push_str(" ORDER BY fts.rank LIMIT ?");
        params.push(limit.to_string());

        let mut query_builder = sqlx::query(&sql);
        for param in &params {
            query_builder = query_builder.bind(param);
        }

        let rows = query_builder
            .fetch_all(&self.pool)
            .await
            .context("Failed to search messages with time filters")?;

        let mut messages = Vec::new();
        for row in rows {
            let message = self.row_to_message(&row)?;
            messages.push(message);
        }

        Ok(messages)
    }

    pub async fn count_by_session(&self, session_id: &Uuid) -> AnyhowResult<i64> {
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM messages WHERE session_id = ?")
            .bind(session_id.to_string())
            .fetch_one(&self.pool)
            .await
            .context("Failed to count messages by session")?;

        Ok(count)
    }

    pub async fn count_all(&self) -> AnyhowResult<i64> {
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM messages")
            .fetch_one(&self.pool)
            .await
            .context("Failed to count all messages")?;

        Ok(count)
    }

    pub async fn delete_by_session(&self, session_id: &Uuid) -> AnyhowResult<u64> {
        let result = sqlx::query("DELETE FROM messages WHERE session_id = ?")
            .bind(session_id.to_string())
            .execute(&self.pool)
            .await
            .context("Failed to delete messages by session")?;

        Ok(result.rows_affected())
    }

    /// Get messages by time range with optional filters
    pub async fn get_by_time_range(
        &self,
        from: Option<DateTime<Utc>>,
        to: Option<DateTime<Utc>>,
        provider: Option<&str>,
        role: Option<&str>,
        limit: Option<i64>,
        reverse: bool,
    ) -> AnyhowResult<Vec<Message>> {
        let mut sql = String::from(
            r#"
            SELECT m.id, m.session_id, m.role, m.content, m.timestamp,
                   m.token_count, m.metadata, m.sequence_number,
                   m.message_type, m.tool_operation_id
            FROM messages m
            "#,
        );

        let mut conditions = Vec::new();

        if from.is_some() {
            conditions.push("m.timestamp >= ?");
        }

        if to.is_some() {
            conditions.push("m.timestamp <= ?");
        }

        if provider.is_some() {
            conditions.push(
                "EXISTS (
                    SELECT 1 FROM chat_sessions cs
                    WHERE cs.id = m.session_id AND cs.provider = ?
                )",
            );
        }

        if role.is_some() {
            conditions.push("m.role = ?");
        }

        if !conditions.is_empty() {
            sql.push_str(" WHERE ");
            sql.push_str(&conditions.join(" AND "));
        }

        sql.push_str(" ORDER BY m.timestamp ");
        sql.push_str(if reverse { "DESC" } else { "ASC" });

        if let Some(lim) = limit {
            sql.push_str(&format!(" LIMIT {lim}"));
        }

        let mut query_builder = sqlx::query(&sql);

        if let Some(from_time) = from {
            query_builder = query_builder.bind(from_time.to_rfc3339());
        }

        if let Some(to_time) = to {
            query_builder = query_builder.bind(to_time.to_rfc3339());
        }

        if let Some(prov) = provider {
            query_builder = query_builder.bind(prov);
        }

        if let Some(r) = role {
            query_builder = query_builder.bind(r);
        }

        let rows = query_builder
            .fetch_all(&self.pool)
            .await
            .context("Failed to fetch messages by time range")?;

        let mut messages = Vec::new();
        for row in rows {
            let message = self.row_to_message(&row)?;
            messages.push(message);
        }

        Ok(messages)
    }

    /// Bulk create messages within a transaction for better performance
    pub async fn bulk_create(&self, messages: &[Message]) -> AnyhowResult<()> {
        if messages.is_empty() {
            return Ok(());
        }

        let mut tx = self
            .pool
            .begin()
            .await
            .context("Failed to start transaction")?;

        for message in messages {
            sqlx::query(
                r#"
                INSERT INTO messages (
                    id, session_id, role, content, timestamp, token_count,
                    metadata, sequence_number, message_type, tool_operation_id
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                "#,
            )
            .bind(message.id.to_string())
            .bind(message.session_id.to_string())
            .bind(message.role.to_string())
            .bind(&message.content)
            .bind(message.timestamp.to_rfc3339())
            .bind(message.token_count)
            .bind("{}") // metadata
            .bind(message.sequence_number)
            .bind(message.message_type.to_string())
            .bind(message.tool_operation_id.map(|id| id.to_string()))
            .execute(&mut *tx)
            .await
            .context("Failed to insert message in bulk")?;
        }

        tx.commit()
            .await
            .context("Failed to commit bulk insert transaction")?;
        Ok(())
    }

    fn row_to_message(&self, row: &SqliteRow) -> AnyhowResult<Message> {
        use crate::models::message::MessageType;

        let id_str: String = row.try_get("id")?;
        let session_id_str: String = row.try_get("session_id")?;
        let role_str: String = row.try_get("role")?;
        let content: String = row.try_get("content")?;
        let timestamp_str: String = row.try_get("timestamp")?;
        let token_count: Option<i64> = row.try_get("token_count")?;
        let sequence_number: i64 = row.try_get("sequence_number")?;
        let message_type_str: String = row.try_get("message_type")?;
        let tool_operation_id_str: Option<String> = row.try_get("tool_operation_id")?;

        let id = Uuid::parse_str(&id_str).context("Invalid message ID format")?;
        let session_id = Uuid::parse_str(&session_id_str).context("Invalid session ID format")?;
        let role = MessageRole::from_str(&role_str)
            .map_err(|e| anyhow::anyhow!("Invalid message role: {e}"))?;
        let timestamp = DateTime::parse_from_rfc3339(&timestamp_str)
            .context("Invalid timestamp format")?
            .with_timezone(&Utc);
        let message_type = MessageType::from_str(&message_type_str)
            .map_err(|e| anyhow::anyhow!("Invalid message type: {e}"))?;
        let tool_operation_id = if let Some(id_str) = tool_operation_id_str {
            Some(Uuid::parse_str(&id_str).context("Invalid tool operation ID format")?)
        } else {
            None
        };

        let metadata: Option<serde_json::Value> = serde_json::from_str("{}").ok();

        Ok(Message {
            id,
            session_id,
            role,
            content,
            timestamp,
            token_count: token_count.map(|tc| tc as u32),
            metadata,
            sequence_number: sequence_number as u32,
            message_type,
            tool_operation_id,
            tool_uses: None,
            tool_results: None,
        })
    }
}
