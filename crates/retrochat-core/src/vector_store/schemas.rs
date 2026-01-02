//! Arrow schema definitions for LanceDB tables.

use std::sync::Arc;

use arrow_schema::{DataType, Field, Schema};

/// Get the Arrow schema for turn embeddings table.
///
/// # Arguments
/// * `dimensions` - Number of dimensions in the embedding vector (e.g., 384)
pub fn turn_embeddings_schema(dimensions: usize) -> Schema {
    Schema::new(vec![
        Field::new("id", DataType::Utf8, false),
        Field::new("session_id", DataType::Utf8, false),
        Field::new("turn_number", DataType::Int32, false),
        Field::new("turn_type", DataType::Utf8, true),
        Field::new(
            "started_at",
            DataType::Timestamp(arrow_schema::TimeUnit::Microsecond, Some("UTC".into())),
            false,
        ),
        Field::new(
            "ended_at",
            DataType::Timestamp(arrow_schema::TimeUnit::Microsecond, Some("UTC".into())),
            false,
        ),
        Field::new(
            "embedding",
            DataType::FixedSizeList(
                Arc::new(Field::new("item", DataType::Float32, false)),
                dimensions as i32,
            ),
            false,
        ),
        Field::new("text_hash", DataType::Utf8, false),
        Field::new(
            "embedded_at",
            DataType::Timestamp(arrow_schema::TimeUnit::Microsecond, Some("UTC".into())),
            false,
        ),
        Field::new("model_name", DataType::Utf8, false),
    ])
}

/// Get the Arrow schema for session embeddings table.
///
/// # Arguments
/// * `dimensions` - Number of dimensions in the embedding vector (e.g., 384)
pub fn session_embeddings_schema(dimensions: usize) -> Schema {
    Schema::new(vec![
        Field::new("id", DataType::Utf8, false),
        Field::new("session_id", DataType::Utf8, false),
        Field::new("outcome", DataType::Utf8, true),
        Field::new(
            "created_at",
            DataType::Timestamp(arrow_schema::TimeUnit::Microsecond, Some("UTC".into())),
            false,
        ),
        Field::new(
            "updated_at",
            DataType::Timestamp(arrow_schema::TimeUnit::Microsecond, Some("UTC".into())),
            false,
        ),
        Field::new("provider", DataType::Utf8, false),
        Field::new("project", DataType::Utf8, true),
        Field::new(
            "embedding",
            DataType::FixedSizeList(
                Arc::new(Field::new("item", DataType::Float32, false)),
                dimensions as i32,
            ),
            false,
        ),
        Field::new("text_hash", DataType::Utf8, false),
        Field::new(
            "embedded_at",
            DataType::Timestamp(arrow_schema::TimeUnit::Microsecond, Some("UTC".into())),
            false,
        ),
        Field::new("model_name", DataType::Utf8, false),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_turn_embeddings_schema() {
        let schema = turn_embeddings_schema(384);

        assert_eq!(schema.fields().len(), 10);
        assert!(schema.field_with_name("id").is_ok());
        assert!(schema.field_with_name("embedding").is_ok());

        let embedding_field = schema.field_with_name("embedding").unwrap();
        match embedding_field.data_type() {
            DataType::FixedSizeList(_, size) => assert_eq!(*size, 384),
            _ => panic!("Expected FixedSizeList"),
        }
    }

    #[test]
    fn test_session_embeddings_schema() {
        let schema = session_embeddings_schema(384);

        assert_eq!(schema.fields().len(), 11);
        assert!(schema.field_with_name("id").is_ok());
        assert!(schema.field_with_name("provider").is_ok());
        assert!(schema.field_with_name("project").is_ok());
        assert!(schema.field_with_name("embedding").is_ok());
    }

    #[test]
    fn test_schema_with_different_dimensions() {
        let schema_384 = turn_embeddings_schema(384);
        let schema_768 = turn_embeddings_schema(768);

        let emb_384 = schema_384.field_with_name("embedding").unwrap();
        let emb_768 = schema_768.field_with_name("embedding").unwrap();

        match (emb_384.data_type(), emb_768.data_type()) {
            (DataType::FixedSizeList(_, s1), DataType::FixedSizeList(_, s2)) => {
                assert_eq!(*s1, 384);
                assert_eq!(*s2, 768);
            }
            _ => panic!("Expected FixedSizeList"),
        }
    }
}
