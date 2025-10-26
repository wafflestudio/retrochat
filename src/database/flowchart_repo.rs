use chrono::{DateTime, Utc};
use std::sync::Arc;

use crate::database::DatabaseManager;
use crate::models::Flowchart;

#[derive(Clone)]
pub struct FlowchartRepository {
    db_manager: Arc<DatabaseManager>,
}

impl FlowchartRepository {
    pub fn new(db_manager: Arc<DatabaseManager>) -> Self {
        Self { db_manager }
    }

    pub async fn create(
        &self,
        flowchart: &Flowchart,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let pool = self.db_manager.pool();

        let created_at_str = flowchart.created_at.to_rfc3339();
        let nodes_json = serde_json::to_string(&flowchart.nodes)?;
        let edges_json = serde_json::to_string(&flowchart.edges)?;

        sqlx::query!(
            r#"
            INSERT INTO flowcharts (
                id, session_id, nodes, edges, created_at, token_usage
            ) VALUES (?, ?, ?, ?, ?, ?)
            "#,
            flowchart.id,
            flowchart.session_id,
            nodes_json,
            edges_json,
            created_at_str,
            flowchart.token_usage
        )
        .execute(pool)
        .await?;

        Ok(())
    }

    pub async fn find_by_id(
        &self,
        id: &str,
    ) -> Result<Option<Flowchart>, Box<dyn std::error::Error + Send + Sync>> {
        let pool = self.db_manager.pool();

        let row = sqlx::query!("SELECT * FROM flowcharts WHERE id = ?", id)
            .fetch_optional(pool)
            .await?;

        if let Some(row) = row {
            let created_at = DateTime::parse_from_rfc3339(&row.created_at)?.with_timezone(&Utc);
            let nodes = serde_json::from_str(&row.nodes)?;
            let edges = serde_json::from_str(&row.edges)?;

            Ok(Some(Flowchart {
                id: row.id.expect("flowchart id should not be null"),
                session_id: row.session_id,
                nodes,
                edges,
                created_at,
                token_usage: row.token_usage.map(|t| t as u32),
            }))
        } else {
            Ok(None)
        }
    }

    pub async fn get_by_session_id(
        &self,
        session_id: &str,
    ) -> Result<Vec<Flowchart>, Box<dyn std::error::Error + Send + Sync>> {
        let pool = self.db_manager.pool();

        let rows = sqlx::query!(
            "SELECT * FROM flowcharts WHERE session_id = ? ORDER BY created_at DESC",
            session_id
        )
        .fetch_all(pool)
        .await?;

        let mut flowcharts = Vec::new();
        for row in rows {
            let created_at = DateTime::parse_from_rfc3339(&row.created_at)?.with_timezone(&Utc);
            let nodes = serde_json::from_str(&row.nodes)?;
            let edges = serde_json::from_str(&row.edges)?;

            flowcharts.push(Flowchart {
                id: row.id.expect("flowchart id should not be null"),
                session_id: row.session_id,
                nodes,
                edges,
                created_at,
                token_usage: row.token_usage.map(|t| t as u32),
            });
        }

        Ok(flowcharts)
    }

    pub async fn delete(&self, id: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let pool = self.db_manager.pool();

        sqlx::query!("DELETE FROM flowcharts WHERE id = ?", id)
            .execute(pool)
            .await?;

        Ok(())
    }

    pub async fn delete_by_session_id(
        &self,
        session_id: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let pool = self.db_manager.pool();

        sqlx::query!("DELETE FROM flowcharts WHERE session_id = ?", session_id)
            .execute(pool)
            .await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::Database;
    use crate::models::{EdgeType, FlowchartEdge, FlowchartNode, MessageRef, NodeType};

    #[tokio::test]
    async fn test_create_and_find_flowchart() {
        let db = Database::new_in_memory().await.unwrap();
        db.initialize().await.unwrap();

        // Create a test session first
        let session_repo = db.chat_session_repo();
        let session = crate::models::ChatSession::new(
            crate::models::Provider::ClaudeCode,
            "test-provider".to_string(),
            "test-hash".to_string(),
            chrono::Utc::now(),
        );
        session_repo.create(&session).await.unwrap();

        let repo = FlowchartRepository::new(Arc::new(db.manager));

        let nodes = vec![FlowchartNode {
            id: "1".to_string(),
            label: "Test node".to_string(),
            message_refs: vec![MessageRef {
                message_id: "msg-1".to_string(),
                sequence_number: 1,
                portion: None,
            }],
            node_type: NodeType::Action,
            description: None,
        }];

        let edges = vec![FlowchartEdge {
            from_node: "1".to_string(),
            to_node: "2".to_string(),
            edge_type: EdgeType::Sequential,
            label: None,
        }];

        let flowchart = Flowchart::new(session.id.to_string(), nodes, edges);
        let flowchart_id = flowchart.id.clone();

        repo.create(&flowchart).await.unwrap();

        let found = repo.find_by_id(&flowchart_id).await.unwrap();
        assert!(found.is_some());

        let found_flowchart = found.unwrap();
        assert_eq!(found_flowchart.session_id, session.id.to_string());
        assert_eq!(found_flowchart.nodes.len(), 1);
        assert_eq!(found_flowchart.edges.len(), 1);
    }

    #[tokio::test]
    async fn test_get_by_session_id() {
        let db = Database::new_in_memory().await.unwrap();
        db.initialize().await.unwrap();

        // Create a test session first
        let session_repo = db.chat_session_repo();
        let session = crate::models::ChatSession::new(
            crate::models::Provider::ClaudeCode,
            "test-provider".to_string(),
            "test-hash".to_string(),
            chrono::Utc::now(),
        );
        session_repo.create(&session).await.unwrap();

        let repo = FlowchartRepository::new(Arc::new(db.manager));

        let flowchart1 = Flowchart::new(session.id.to_string(), vec![], vec![]);
        let flowchart2 = Flowchart::new(session.id.to_string(), vec![], vec![]);

        repo.create(&flowchart1).await.unwrap();
        repo.create(&flowchart2).await.unwrap();

        let flowcharts = repo
            .get_by_session_id(&session.id.to_string())
            .await
            .unwrap();
        assert_eq!(flowcharts.len(), 2);
    }
}
