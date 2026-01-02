//! LanceDB vector store implementation.

use std::path::Path;
use std::sync::Arc;

use anyhow::{Context, Result};
use arrow_array::{
    Array, FixedSizeListArray, Float32Array, Int32Array, RecordBatch, RecordBatchIterator,
    StringArray, TimestampMicrosecondArray,
};
use arrow_schema::Schema;
use chrono::{DateTime, TimeZone, Utc};
use futures::TryStreamExt;
use lancedb::query::{ExecutableQuery, QueryBase};
use lancedb::{connect, Connection, Table};

use super::models::{
    SessionEmbedding, SessionFilter, SessionSearchResult, TurnEmbedding, TurnFilter,
    TurnSearchResult, VectorStoreStats,
};
use super::schemas::{session_embeddings_schema, turn_embeddings_schema};

const TURN_TABLE_NAME: &str = "turn_embeddings";
const SESSION_TABLE_NAME: &str = "session_embeddings";

/// Vector store using LanceDB for semantic search.
pub struct VectorStore {
    connection: Connection,
    dimensions: usize,
}

impl VectorStore {
    /// Open or create a vector store at the given path.
    ///
    /// # Arguments
    /// * `path` - Directory path for the LanceDB database
    /// * `dimensions` - Number of dimensions in embedding vectors (e.g., 384)
    pub async fn open(path: impl AsRef<Path>, dimensions: usize) -> Result<Self> {
        let path_str = path.as_ref().to_string_lossy().to_string();

        // Ensure directory exists
        std::fs::create_dir_all(path.as_ref())
            .context("Failed to create vector store directory")?;

        let connection = connect(&path_str)
            .execute()
            .await
            .context("Failed to connect to LanceDB")?;

        let store = Self {
            connection,
            dimensions,
        };

        // Ensure tables exist
        store.ensure_tables().await?;

        Ok(store)
    }

    /// Ensure required tables exist, creating them if necessary.
    async fn ensure_tables(&self) -> Result<()> {
        let tables = self
            .connection
            .table_names()
            .execute()
            .await
            .context("Failed to list tables")?;

        if !tables.contains(&TURN_TABLE_NAME.to_string()) {
            self.create_turn_table().await?;
        }

        if !tables.contains(&SESSION_TABLE_NAME.to_string()) {
            self.create_session_table().await?;
        }

        Ok(())
    }

    /// Create the turn embeddings table.
    async fn create_turn_table(&self) -> Result<()> {
        let schema = Arc::new(turn_embeddings_schema(self.dimensions));
        let empty_batch = self.create_empty_turn_batch(&schema)?;

        let batches = RecordBatchIterator::new(vec![Ok(empty_batch)], schema);

        self.connection
            .create_table(TURN_TABLE_NAME, Box::new(batches))
            .execute()
            .await
            .context("Failed to create turn_embeddings table")?;

        Ok(())
    }

    /// Create the session embeddings table.
    async fn create_session_table(&self) -> Result<()> {
        let schema = Arc::new(session_embeddings_schema(self.dimensions));
        let empty_batch = self.create_empty_session_batch(&schema)?;

        let batches = RecordBatchIterator::new(vec![Ok(empty_batch)], schema);

        self.connection
            .create_table(SESSION_TABLE_NAME, Box::new(batches))
            .execute()
            .await
            .context("Failed to create session_embeddings table")?;

        Ok(())
    }

    /// Create an empty record batch for turn embeddings (used for table creation).
    fn create_empty_turn_batch(&self, schema: &Arc<Schema>) -> Result<RecordBatch> {
        let id: StringArray = (vec![] as Vec<&str>).into();
        let session_id: StringArray = (vec![] as Vec<&str>).into();
        let turn_number: Int32Array = (vec![] as Vec<i32>).into();
        let turn_type: StringArray = StringArray::from(vec![] as Vec<Option<&str>>);
        let started_at =
            TimestampMicrosecondArray::from(vec![] as Vec<i64>).with_timezone("UTC".to_string());
        let ended_at =
            TimestampMicrosecondArray::from(vec![] as Vec<i64>).with_timezone("UTC".to_string());
        let embedding = FixedSizeListArray::new(
            Arc::new(arrow_schema::Field::new(
                "item",
                arrow_schema::DataType::Float32,
                false,
            )),
            self.dimensions as i32,
            Arc::new(Float32Array::from(vec![] as Vec<f32>)),
            None,
        );
        let text_hash: StringArray = (vec![] as Vec<&str>).into();
        let embedded_at =
            TimestampMicrosecondArray::from(vec![] as Vec<i64>).with_timezone("UTC".to_string());
        let model_name: StringArray = (vec![] as Vec<&str>).into();

        RecordBatch::try_new(
            schema.clone(),
            vec![
                Arc::new(id),
                Arc::new(session_id),
                Arc::new(turn_number),
                Arc::new(turn_type),
                Arc::new(started_at),
                Arc::new(ended_at),
                Arc::new(embedding),
                Arc::new(text_hash),
                Arc::new(embedded_at),
                Arc::new(model_name),
            ],
        )
        .context("Failed to create empty turn batch")
    }

    /// Create an empty record batch for session embeddings (used for table creation).
    fn create_empty_session_batch(&self, schema: &Arc<Schema>) -> Result<RecordBatch> {
        let id: StringArray = (vec![] as Vec<&str>).into();
        let session_id: StringArray = (vec![] as Vec<&str>).into();
        let outcome: StringArray = StringArray::from(vec![] as Vec<Option<&str>>);
        let created_at =
            TimestampMicrosecondArray::from(vec![] as Vec<i64>).with_timezone("UTC".to_string());
        let updated_at =
            TimestampMicrosecondArray::from(vec![] as Vec<i64>).with_timezone("UTC".to_string());
        let provider: StringArray = (vec![] as Vec<&str>).into();
        let project: StringArray = StringArray::from(vec![] as Vec<Option<&str>>);
        let embedding = FixedSizeListArray::new(
            Arc::new(arrow_schema::Field::new(
                "item",
                arrow_schema::DataType::Float32,
                false,
            )),
            self.dimensions as i32,
            Arc::new(Float32Array::from(vec![] as Vec<f32>)),
            None,
        );
        let text_hash: StringArray = (vec![] as Vec<&str>).into();
        let embedded_at =
            TimestampMicrosecondArray::from(vec![] as Vec<i64>).with_timezone("UTC".to_string());
        let model_name: StringArray = (vec![] as Vec<&str>).into();

        RecordBatch::try_new(
            schema.clone(),
            vec![
                Arc::new(id),
                Arc::new(session_id),
                Arc::new(outcome),
                Arc::new(created_at),
                Arc::new(updated_at),
                Arc::new(provider),
                Arc::new(project),
                Arc::new(embedding),
                Arc::new(text_hash),
                Arc::new(embedded_at),
                Arc::new(model_name),
            ],
        )
        .context("Failed to create empty session batch")
    }

    /// Get the turn embeddings table.
    async fn turn_table(&self) -> Result<Table> {
        self.connection
            .open_table(TURN_TABLE_NAME)
            .execute()
            .await
            .context("Failed to open turn_embeddings table")
    }

    /// Get the session embeddings table.
    async fn session_table(&self) -> Result<Table> {
        self.connection
            .open_table(SESSION_TABLE_NAME)
            .execute()
            .await
            .context("Failed to open session_embeddings table")
    }

    /// Upsert a turn embedding.
    pub async fn upsert_turn_embedding(&self, embedding: TurnEmbedding) -> Result<()> {
        // Delete existing if present
        self.delete_turn_embedding(&embedding.id).await.ok();

        let table = self.turn_table().await?;
        let schema = Arc::new(turn_embeddings_schema(self.dimensions));
        let batch = self.turn_embedding_to_batch(&embedding, &schema)?;

        let batches = RecordBatchIterator::new(vec![Ok(batch)], schema);

        table
            .add(Box::new(batches))
            .execute()
            .await
            .context("Failed to upsert turn embedding")?;

        Ok(())
    }

    /// Convert a TurnEmbedding to a RecordBatch.
    fn turn_embedding_to_batch(
        &self,
        emb: &TurnEmbedding,
        schema: &Arc<Schema>,
    ) -> Result<RecordBatch> {
        let id = StringArray::from(vec![emb.id.as_str()]);
        let session_id = StringArray::from(vec![emb.session_id.as_str()]);
        let turn_number = Int32Array::from(vec![emb.turn_number]);
        let turn_type = StringArray::from(vec![emb.turn_type.as_deref()]);
        let started_at = TimestampMicrosecondArray::from(vec![emb.started_at.timestamp_micros()])
            .with_timezone("UTC".to_string());
        let ended_at = TimestampMicrosecondArray::from(vec![emb.ended_at.timestamp_micros()])
            .with_timezone("UTC".to_string());

        let embedding_values = Float32Array::from(emb.embedding.clone());
        let embedding = FixedSizeListArray::new(
            Arc::new(arrow_schema::Field::new(
                "item",
                arrow_schema::DataType::Float32,
                false,
            )),
            self.dimensions as i32,
            Arc::new(embedding_values),
            None,
        );

        let text_hash = StringArray::from(vec![emb.text_hash.as_str()]);
        let embedded_at = TimestampMicrosecondArray::from(vec![emb.embedded_at.timestamp_micros()])
            .with_timezone("UTC".to_string());
        let model_name = StringArray::from(vec![emb.model_name.as_str()]);

        RecordBatch::try_new(
            schema.clone(),
            vec![
                Arc::new(id),
                Arc::new(session_id),
                Arc::new(turn_number),
                Arc::new(turn_type),
                Arc::new(started_at),
                Arc::new(ended_at),
                Arc::new(embedding),
                Arc::new(text_hash),
                Arc::new(embedded_at),
                Arc::new(model_name),
            ],
        )
        .context("Failed to create turn embedding batch")
    }

    /// Upsert a session embedding.
    pub async fn upsert_session_embedding(&self, embedding: SessionEmbedding) -> Result<()> {
        // Delete existing if present
        self.delete_session_embedding(&embedding.id).await.ok();

        let table = self.session_table().await?;
        let schema = Arc::new(session_embeddings_schema(self.dimensions));
        let batch = self.session_embedding_to_batch(&embedding, &schema)?;

        let batches = RecordBatchIterator::new(vec![Ok(batch)], schema);

        table
            .add(Box::new(batches))
            .execute()
            .await
            .context("Failed to upsert session embedding")?;

        Ok(())
    }

    /// Convert a SessionEmbedding to a RecordBatch.
    fn session_embedding_to_batch(
        &self,
        emb: &SessionEmbedding,
        schema: &Arc<Schema>,
    ) -> Result<RecordBatch> {
        let id = StringArray::from(vec![emb.id.as_str()]);
        let session_id = StringArray::from(vec![emb.session_id.as_str()]);
        let outcome = StringArray::from(vec![emb.outcome.as_deref()]);
        let created_at = TimestampMicrosecondArray::from(vec![emb.created_at.timestamp_micros()])
            .with_timezone("UTC".to_string());
        let updated_at = TimestampMicrosecondArray::from(vec![emb.updated_at.timestamp_micros()])
            .with_timezone("UTC".to_string());
        let provider = StringArray::from(vec![emb.provider.as_str()]);
        let project = StringArray::from(vec![emb.project.as_deref()]);

        let embedding_values = Float32Array::from(emb.embedding.clone());
        let embedding = FixedSizeListArray::new(
            Arc::new(arrow_schema::Field::new(
                "item",
                arrow_schema::DataType::Float32,
                false,
            )),
            self.dimensions as i32,
            Arc::new(embedding_values),
            None,
        );

        let text_hash = StringArray::from(vec![emb.text_hash.as_str()]);
        let embedded_at = TimestampMicrosecondArray::from(vec![emb.embedded_at.timestamp_micros()])
            .with_timezone("UTC".to_string());
        let model_name = StringArray::from(vec![emb.model_name.as_str()]);

        RecordBatch::try_new(
            schema.clone(),
            vec![
                Arc::new(id),
                Arc::new(session_id),
                Arc::new(outcome),
                Arc::new(created_at),
                Arc::new(updated_at),
                Arc::new(provider),
                Arc::new(project),
                Arc::new(embedding),
                Arc::new(text_hash),
                Arc::new(embedded_at),
                Arc::new(model_name),
            ],
        )
        .context("Failed to create session embedding batch")
    }

    /// Delete a turn embedding by ID.
    pub async fn delete_turn_embedding(&self, id: &str) -> Result<()> {
        let table = self.turn_table().await?;
        table
            .delete(&format!("id = '{}'", id))
            .await
            .context("Failed to delete turn embedding")?;
        Ok(())
    }

    /// Delete a session embedding by ID.
    pub async fn delete_session_embedding(&self, id: &str) -> Result<()> {
        let table = self.session_table().await?;
        table
            .delete(&format!("id = '{}'", id))
            .await
            .context("Failed to delete session embedding")?;
        Ok(())
    }

    /// Delete all embeddings for a session.
    pub async fn delete_session(&self, session_id: &str) -> Result<()> {
        let turn_table = self.turn_table().await?;
        turn_table
            .delete(&format!("session_id = '{}'", session_id))
            .await
            .ok();

        let session_table = self.session_table().await?;
        session_table
            .delete(&format!("session_id = '{}'", session_id))
            .await
            .ok();

        Ok(())
    }

    /// Get a turn embedding by ID.
    pub async fn get_turn_embedding(&self, id: &str) -> Result<Option<TurnEmbedding>> {
        let table = self.turn_table().await?;

        let mut stream = table
            .query()
            .only_if(format!("id = '{}'", id))
            .limit(1)
            .execute()
            .await
            .context("Failed to query turn embedding")?;

        if let Some(batch) = stream.try_next().await? {
            if batch.num_rows() > 0 {
                return Ok(Some(self.batch_to_turn_embedding(&batch, 0)?));
            }
        }

        Ok(None)
    }

    /// Get a session embedding by ID.
    pub async fn get_session_embedding(&self, id: &str) -> Result<Option<SessionEmbedding>> {
        let table = self.session_table().await?;

        let mut stream = table
            .query()
            .only_if(format!("id = '{}'", id))
            .limit(1)
            .execute()
            .await
            .context("Failed to query session embedding")?;

        if let Some(batch) = stream.try_next().await? {
            if batch.num_rows() > 0 {
                return Ok(Some(self.batch_to_session_embedding(&batch, 0)?));
            }
        }

        Ok(None)
    }

    /// Search turn embeddings by vector similarity.
    pub async fn search_turns(
        &self,
        query_vector: &[f32],
        limit: usize,
        filter: Option<TurnFilter>,
    ) -> Result<Vec<TurnSearchResult>> {
        let table = self.turn_table().await?;

        let mut query = table.vector_search(query_vector.to_vec())?.limit(limit);

        if let Some(f) = filter {
            if let Some(sql) = f.to_sql() {
                query = query.only_if(sql);
            }
        }

        let mut stream = query.execute().await.context("Failed to search turns")?;

        let mut results = Vec::new();
        while let Some(batch) = stream.try_next().await? {
            for i in 0..batch.num_rows() {
                let id = batch
                    .column_by_name("id")
                    .and_then(|c| c.as_any().downcast_ref::<StringArray>())
                    .and_then(|a| a.value(i).to_string().into())
                    .unwrap_or_default();

                let session_id = batch
                    .column_by_name("session_id")
                    .and_then(|c| c.as_any().downcast_ref::<StringArray>())
                    .and_then(|a| a.value(i).to_string().into())
                    .unwrap_or_default();

                let turn_number = batch
                    .column_by_name("turn_number")
                    .and_then(|c| c.as_any().downcast_ref::<Int32Array>())
                    .map(|a| a.value(i))
                    .unwrap_or(0);

                let score = batch
                    .column_by_name("_distance")
                    .and_then(|c| c.as_any().downcast_ref::<Float32Array>())
                    .map(|a| 1.0 - a.value(i)) // Convert distance to similarity
                    .unwrap_or(0.0);

                results.push(TurnSearchResult {
                    id,
                    session_id,
                    turn_number,
                    score,
                });
            }
        }

        Ok(results)
    }

    /// Search session embeddings by vector similarity.
    pub async fn search_sessions(
        &self,
        query_vector: &[f32],
        limit: usize,
        filter: Option<SessionFilter>,
    ) -> Result<Vec<SessionSearchResult>> {
        let table = self.session_table().await?;

        let mut query = table.vector_search(query_vector.to_vec())?.limit(limit);

        if let Some(f) = filter {
            if let Some(sql) = f.to_sql() {
                query = query.only_if(sql);
            }
        }

        let mut stream = query.execute().await.context("Failed to search sessions")?;

        let mut results = Vec::new();
        while let Some(batch) = stream.try_next().await? {
            for i in 0..batch.num_rows() {
                let id = batch
                    .column_by_name("id")
                    .and_then(|c| c.as_any().downcast_ref::<StringArray>())
                    .and_then(|a| a.value(i).to_string().into())
                    .unwrap_or_default();

                let session_id = batch
                    .column_by_name("session_id")
                    .and_then(|c| c.as_any().downcast_ref::<StringArray>())
                    .and_then(|a| a.value(i).to_string().into())
                    .unwrap_or_default();

                let score = batch
                    .column_by_name("_distance")
                    .and_then(|c| c.as_any().downcast_ref::<Float32Array>())
                    .map(|a| 1.0 - a.value(i)) // Convert distance to similarity
                    .unwrap_or(0.0);

                results.push(SessionSearchResult {
                    id,
                    session_id,
                    score,
                });
            }
        }

        Ok(results)
    }

    /// Get statistics about the vector store.
    pub async fn get_stats(&self) -> Result<VectorStoreStats> {
        let turn_table = self.turn_table().await?;
        let session_table = self.session_table().await?;

        let turn_count = turn_table.count_rows(None).await.unwrap_or(0) as usize;
        let session_count = session_table.count_rows(None).await.unwrap_or(0) as usize;

        Ok(VectorStoreStats {
            turn_count,
            session_count,
            dimensions: self.dimensions,
            model_name: None, // Would need to query a row to get this
        })
    }

    /// Convert a RecordBatch row to TurnEmbedding.
    fn batch_to_turn_embedding(&self, batch: &RecordBatch, row: usize) -> Result<TurnEmbedding> {
        let id = batch
            .column_by_name("id")
            .and_then(|c| c.as_any().downcast_ref::<StringArray>())
            .map(|a| a.value(row).to_string())
            .unwrap_or_default();

        let session_id = batch
            .column_by_name("session_id")
            .and_then(|c| c.as_any().downcast_ref::<StringArray>())
            .map(|a| a.value(row).to_string())
            .unwrap_or_default();

        let turn_number = batch
            .column_by_name("turn_number")
            .and_then(|c| c.as_any().downcast_ref::<Int32Array>())
            .map(|a| a.value(row))
            .unwrap_or(0);

        let turn_type = batch
            .column_by_name("turn_type")
            .and_then(|c| c.as_any().downcast_ref::<StringArray>())
            .and_then(|a| {
                if a.is_null(row) {
                    None
                } else {
                    Some(a.value(row).to_string())
                }
            });

        let started_at = batch
            .column_by_name("started_at")
            .and_then(|c| c.as_any().downcast_ref::<TimestampMicrosecondArray>())
            .map(|a| Utc.timestamp_micros(a.value(row)).unwrap())
            .unwrap_or_else(Utc::now);

        let ended_at = batch
            .column_by_name("ended_at")
            .and_then(|c| c.as_any().downcast_ref::<TimestampMicrosecondArray>())
            .map(|a| Utc.timestamp_micros(a.value(row)).unwrap())
            .unwrap_or_else(Utc::now);

        let embedding = batch
            .column_by_name("embedding")
            .and_then(|c| c.as_any().downcast_ref::<FixedSizeListArray>())
            .map(|a| {
                let values = a.value(row);
                values
                    .as_any()
                    .downcast_ref::<Float32Array>()
                    .map(|fa| fa.values().to_vec())
                    .unwrap_or_default()
            })
            .unwrap_or_default();

        let text_hash = batch
            .column_by_name("text_hash")
            .and_then(|c| c.as_any().downcast_ref::<StringArray>())
            .map(|a| a.value(row).to_string())
            .unwrap_or_default();

        let embedded_at = batch
            .column_by_name("embedded_at")
            .and_then(|c| c.as_any().downcast_ref::<TimestampMicrosecondArray>())
            .map(|a| Utc.timestamp_micros(a.value(row)).unwrap())
            .unwrap_or_else(Utc::now);

        let model_name = batch
            .column_by_name("model_name")
            .and_then(|c| c.as_any().downcast_ref::<StringArray>())
            .map(|a| a.value(row).to_string())
            .unwrap_or_default();

        Ok(TurnEmbedding {
            id,
            session_id,
            turn_number,
            turn_type,
            started_at,
            ended_at,
            embedding,
            text_hash,
            embedded_at,
            model_name,
        })
    }

    /// Convert a RecordBatch row to SessionEmbedding.
    fn batch_to_session_embedding(
        &self,
        batch: &RecordBatch,
        row: usize,
    ) -> Result<SessionEmbedding> {
        let id = batch
            .column_by_name("id")
            .and_then(|c| c.as_any().downcast_ref::<StringArray>())
            .map(|a| a.value(row).to_string())
            .unwrap_or_default();

        let session_id = batch
            .column_by_name("session_id")
            .and_then(|c| c.as_any().downcast_ref::<StringArray>())
            .map(|a| a.value(row).to_string())
            .unwrap_or_default();

        let outcome = batch
            .column_by_name("outcome")
            .and_then(|c| c.as_any().downcast_ref::<StringArray>())
            .and_then(|a| {
                if a.is_null(row) {
                    None
                } else {
                    Some(a.value(row).to_string())
                }
            });

        let created_at = batch
            .column_by_name("created_at")
            .and_then(|c| c.as_any().downcast_ref::<TimestampMicrosecondArray>())
            .map(|a| Utc.timestamp_micros(a.value(row)).unwrap())
            .unwrap_or_else(Utc::now);

        let updated_at = batch
            .column_by_name("updated_at")
            .and_then(|c| c.as_any().downcast_ref::<TimestampMicrosecondArray>())
            .map(|a| Utc.timestamp_micros(a.value(row)).unwrap())
            .unwrap_or_else(Utc::now);

        let provider = batch
            .column_by_name("provider")
            .and_then(|c| c.as_any().downcast_ref::<StringArray>())
            .map(|a| a.value(row).to_string())
            .unwrap_or_default();

        let project = batch
            .column_by_name("project")
            .and_then(|c| c.as_any().downcast_ref::<StringArray>())
            .and_then(|a| {
                if a.is_null(row) {
                    None
                } else {
                    Some(a.value(row).to_string())
                }
            });

        let embedding = batch
            .column_by_name("embedding")
            .and_then(|c| c.as_any().downcast_ref::<FixedSizeListArray>())
            .map(|a| {
                let values = a.value(row);
                values
                    .as_any()
                    .downcast_ref::<Float32Array>()
                    .map(|fa| fa.values().to_vec())
                    .unwrap_or_default()
            })
            .unwrap_or_default();

        let text_hash = batch
            .column_by_name("text_hash")
            .and_then(|c| c.as_any().downcast_ref::<StringArray>())
            .map(|a| a.value(row).to_string())
            .unwrap_or_default();

        let embedded_at = batch
            .column_by_name("embedded_at")
            .and_then(|c| c.as_any().downcast_ref::<TimestampMicrosecondArray>())
            .map(|a| Utc.timestamp_micros(a.value(row)).unwrap())
            .unwrap_or_else(Utc::now);

        let model_name = batch
            .column_by_name("model_name")
            .and_then(|c| c.as_any().downcast_ref::<StringArray>())
            .map(|a| a.value(row).to_string())
            .unwrap_or_default();

        Ok(SessionEmbedding {
            id,
            session_id,
            outcome,
            created_at,
            updated_at,
            provider,
            project,
            embedding,
            text_hash,
            embedded_at,
            model_name,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    async fn create_test_store() -> (VectorStore, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let store = VectorStore::open(temp_dir.path(), 384).await.unwrap();
        (store, temp_dir)
    }

    #[tokio::test]
    async fn test_create_store() {
        let (store, _temp_dir) = create_test_store().await;
        let stats = store.get_stats().await.unwrap();

        assert_eq!(stats.turn_count, 0);
        assert_eq!(stats.session_count, 0);
        assert_eq!(stats.dimensions, 384);
    }

    #[tokio::test]
    async fn test_upsert_turn_embedding() {
        let (store, _temp_dir) = create_test_store().await;

        let embedding = TurnEmbedding {
            id: "turn-1".to_string(),
            session_id: "session-1".to_string(),
            turn_number: 1,
            turn_type: Some("task".to_string()),
            started_at: Utc::now(),
            ended_at: Utc::now(),
            embedding: vec![0.1; 384],
            text_hash: "abc123".to_string(),
            embedded_at: Utc::now(),
            model_name: "test-model".to_string(),
        };

        store
            .upsert_turn_embedding(embedding.clone())
            .await
            .unwrap();

        let retrieved = store.get_turn_embedding("turn-1").await.unwrap().unwrap();
        assert_eq!(retrieved.id, "turn-1");
        assert_eq!(retrieved.session_id, "session-1");
        assert_eq!(retrieved.turn_number, 1);
        assert_eq!(retrieved.embedding.len(), 384);
    }

    #[tokio::test]
    async fn test_upsert_session_embedding() {
        let (store, _temp_dir) = create_test_store().await;

        let embedding = SessionEmbedding {
            id: "sess-emb-1".to_string(),
            session_id: "session-1".to_string(),
            outcome: Some("completed".to_string()),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            provider: "claude".to_string(),
            project: Some("my-project".to_string()),
            embedding: vec![0.2; 384],
            text_hash: "def456".to_string(),
            embedded_at: Utc::now(),
            model_name: "test-model".to_string(),
        };

        store
            .upsert_session_embedding(embedding.clone())
            .await
            .unwrap();

        let retrieved = store
            .get_session_embedding("sess-emb-1")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(retrieved.id, "sess-emb-1");
        assert_eq!(retrieved.provider, "claude");
        assert_eq!(retrieved.project, Some("my-project".to_string()));
    }

    #[tokio::test]
    async fn test_delete_session() {
        let (store, _temp_dir) = create_test_store().await;

        // Add turn embedding
        let turn = TurnEmbedding {
            id: "turn-1".to_string(),
            session_id: "session-to-delete".to_string(),
            turn_number: 1,
            turn_type: None,
            started_at: Utc::now(),
            ended_at: Utc::now(),
            embedding: vec![0.1; 384],
            text_hash: "hash".to_string(),
            embedded_at: Utc::now(),
            model_name: "test".to_string(),
        };
        store.upsert_turn_embedding(turn).await.unwrap();

        // Add session embedding
        let session = SessionEmbedding {
            id: "sess-1".to_string(),
            session_id: "session-to-delete".to_string(),
            outcome: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            provider: "claude".to_string(),
            project: None,
            embedding: vec![0.2; 384],
            text_hash: "hash".to_string(),
            embedded_at: Utc::now(),
            model_name: "test".to_string(),
        };
        store.upsert_session_embedding(session).await.unwrap();

        // Delete all for session
        store.delete_session("session-to-delete").await.unwrap();

        // Verify deleted
        assert!(store.get_turn_embedding("turn-1").await.unwrap().is_none());
        assert!(store
            .get_session_embedding("sess-1")
            .await
            .unwrap()
            .is_none());
    }
}
