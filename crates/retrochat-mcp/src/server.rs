//! MCP Server implementation for RetroChat

use crate::error::{not_found_error, to_mcp_error, validation_error};
use retrochat_core::database::DatabaseManager;
use retrochat_core::services::{
    DateRange, QueryService, SearchRequest, SessionDetailRequest, SessionFilters,
    SessionsQueryRequest,
};
use rmcp::handler::server::{router::tool::ToolRouter, wrapper::Parameters};
use rmcp::model::{CallToolResult, Content, ServerCapabilities, ServerInfo};
use rmcp::{tool, tool_handler, tool_router, ErrorData as McpError, ServerHandler};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

/// RetroChat MCP Server
///
/// Provides read-only access to chat session data and analytics
/// through the Model Context Protocol.
#[derive(Clone)]
pub struct RetroChatMcpServer {
    pub(crate) db_manager: Arc<DatabaseManager>,
    pub(crate) tool_router: ToolRouter<Self>,
}

impl RetroChatMcpServer {
    /// Get the query service (creates fresh instance)
    pub(crate) fn query_service(&self) -> QueryService {
        QueryService::with_database(self.db_manager.clone())
    }

    /// Create a new MCP server with default database
    pub async fn new() -> anyhow::Result<Self> {
        let db_path = retrochat_core::database::config::get_default_db_path()?;
        let db_manager = Arc::new(DatabaseManager::new(&db_path).await?);

        Ok(Self {
            db_manager,
            tool_router: Self::tool_router(),
        })
    }

    /// Create a new MCP server with a specific database (for testing)
    pub async fn with_database(db_manager: Arc<DatabaseManager>) -> Self {
        Self {
            db_manager,
            tool_router: Self::tool_router(),
        }
    }
}

// Implement the ServerHandler trait
#[tool_handler(router = self.tool_router)]
impl ServerHandler for RetroChatMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            server_info: rmcp::model::Implementation {
                name: "retrochat-mcp".into(),
                version: env!("CARGO_PKG_VERSION").into(),
                title: Some("RetroChat MCP Server".into()),
                website_url: None,
                icons: None,
            },
            instructions: Some(
                "RetroChat MCP Server - Query and analyze your AI chat history. \
                 Use list_sessions to browse sessions, get_session_detail for full session info, \
                 search_messages for full-text search, and get_session_analytics for analytics data."
                    .into(),
            ),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_server_initialization() {
        let db_manager = Arc::new(DatabaseManager::open_in_memory().await.unwrap());
        let _server = RetroChatMcpServer::with_database(db_manager.clone()).await;

        // Verify shared database manager
        assert!(Arc::strong_count(&db_manager) >= 2); // server + our reference
    }

    #[tokio::test]
    async fn test_server_info() {
        let db_manager = Arc::new(DatabaseManager::open_in_memory().await.unwrap());
        let server = RetroChatMcpServer::with_database(db_manager).await;
        let info = server.get_info();

        assert_eq!(info.server_info.name, "retrochat-mcp");
        assert_eq!(info.server_info.version, env!("CARGO_PKG_VERSION"));
        assert!(info.instructions.is_some());
        assert!(info.instructions.unwrap().contains("RetroChat MCP Server"));
        assert!(info.capabilities.tools.is_some());
    }

    #[tokio::test]
    async fn test_server_clone() {
        let db_manager = Arc::new(DatabaseManager::open_in_memory().await.unwrap());
        let server = RetroChatMcpServer::with_database(db_manager.clone()).await;
        let cloned = server.clone();

        // Both should share the same database manager
        assert!(Arc::ptr_eq(&server.db_manager, &cloned.db_manager));
    }

    #[tokio::test]
    async fn test_server_capabilities() {
        let db_manager = Arc::new(DatabaseManager::open_in_memory().await.unwrap());
        let server = RetroChatMcpServer::with_database(db_manager).await;
        let info = server.get_info();

        assert!(info.capabilities.tools.is_some());
        // Read-only server - no prompts, resources, or sampling
        assert!(info.capabilities.prompts.is_none());
        assert!(info.capabilities.resources.is_none());
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ListSessionsParams {
    /// Filter by provider (e.g., "Claude Code", "Gemini CLI")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,

    /// Filter by project name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project: Option<String>,

    /// Filter sessions from this date (ISO 8601 format)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_date: Option<String>,

    /// Filter sessions until this date (ISO 8601 format)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_date: Option<String>,

    /// Minimum message count
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_messages: Option<i32>,

    /// Maximum message count
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_messages: Option<i32>,

    /// Page number (default: 1)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page: Option<i32>,

    /// Items per page (default: 20)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_size: Option<i32>,

    /// Sort field (default: "start_time")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sort_by: Option<String>,

    /// Sort order: "asc" or "desc" (default: "desc")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sort_order: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GetSessionDetailParams {
    /// Session ID (UUID format)
    pub session_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SearchMessagesParams {
    /// Search query string
    pub query: String,

    /// Filter by providers (e.g., ["Claude Code", "Gemini CLI"])
    #[serde(skip_serializing_if = "Option::is_none")]
    pub providers: Option<Vec<String>>,

    /// Filter by projects
    #[serde(skip_serializing_if = "Option::is_none")]
    pub projects: Option<Vec<String>>,

    /// Search from this date (ISO 8601 format)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_date: Option<String>,

    /// Search until this date (ISO 8601 format)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_date: Option<String>,

    /// Page number (default: 1)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page: Option<i32>,

    /// Items per page (default: 20)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_size: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GetSessionAnalyticsParams {
    /// Session ID (UUID format)
    pub session_id: String,
}

// ============================================================================
// Tool Implementations
// ============================================================================

#[tool_router(router = tool_router)]
impl RetroChatMcpServer {
    /// List chat sessions with optional filtering and pagination
    #[tool(
        description = "List chat sessions with optional filtering by provider, project, date range, message count, and pagination support"
    )]
    pub async fn list_sessions(
        &self,
        params: Parameters<ListSessionsParams>,
    ) -> Result<CallToolResult, McpError> {
        let params = params.0;

        // Validate dates
        if let Some(ref start) = params.start_date {
            chrono::DateTime::parse_from_rfc3339(start)
                .map_err(|_| validation_error(&format!("Invalid start_date format: {}", start)))?;
        }
        if let Some(ref end) = params.end_date {
            chrono::DateTime::parse_from_rfc3339(end)
                .map_err(|_| validation_error(&format!("Invalid end_date format: {}", end)))?;
        }
        if let Some(ref order) = params.sort_order {
            if order != "asc" && order != "desc" {
                return Err(validation_error(&format!(
                    "Invalid sort_order: {}. Must be 'asc' or 'desc'",
                    order
                )));
            }
        }

        // Build request
        let date_range = match (&params.start_date, &params.end_date) {
            (Some(start), Some(end)) => Some(DateRange {
                start_date: start.clone(),
                end_date: end.clone(),
            }),
            _ => None,
        };

        let filters = if params.provider.is_some()
            || params.project.is_some()
            || date_range.is_some()
            || params.min_messages.is_some()
            || params.max_messages.is_some()
        {
            Some(SessionFilters {
                provider: params.provider,
                project: params.project,
                date_range,
                min_messages: params.min_messages,
                max_messages: params.max_messages,
            })
        } else {
            None
        };

        let request = SessionsQueryRequest {
            page: params.page,
            page_size: params.page_size,
            sort_by: params.sort_by,
            sort_order: params.sort_order,
            filters,
        };

        // Query sessions
        let response = self
            .query_service()
            .query_sessions(request)
            .await
            .map_err(to_mcp_error)?;

        // Return pretty-printed JSON
        let json = serde_json::to_string_pretty(&response)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    /// Get detailed information about a specific chat session including all messages
    #[tool(
        description = "Get detailed information about a specific chat session including all messages, tool usage, and metadata"
    )]
    pub async fn get_session_detail(
        &self,
        params: Parameters<GetSessionDetailParams>,
    ) -> Result<CallToolResult, McpError> {
        let params = params.0;

        // Validate UUID format
        Uuid::parse_str(&params.session_id).map_err(|_| {
            validation_error(&format!(
                "Invalid session_id format: {}. Must be a valid UUID",
                params.session_id
            ))
        })?;

        // Create request
        let request = SessionDetailRequest {
            session_id: params.session_id.clone(),
            include_content: Some(true),
            message_limit: None,
            message_offset: None,
        };

        // Get session detail
        let response = self
            .query_service()
            .get_session_detail(request)
            .await
            .map_err(|e| {
                let err_msg = e.to_string();
                if err_msg.contains("not found") || err_msg.contains("Session not found") {
                    not_found_error(&params.session_id)
                } else {
                    to_mcp_error(e)
                }
            })?;

        // Return pretty-printed JSON
        let json = serde_json::to_string_pretty(&response)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    /// Full-text search across all messages in chat sessions
    #[tool(
        description = "Search for messages across all chat sessions using full-text search. Supports filtering by providers, projects, and date ranges"
    )]
    pub async fn search_messages(
        &self,
        params: Parameters<SearchMessagesParams>,
    ) -> Result<CallToolResult, McpError> {
        let params = params.0;

        // Validate query
        if params.query.trim().is_empty() {
            return Err(validation_error("Search query cannot be empty"));
        }

        // Validate dates
        if let Some(ref start) = params.start_date {
            chrono::DateTime::parse_from_rfc3339(start)
                .map_err(|_| validation_error(&format!("Invalid start_date format: {}", start)))?;
        }
        if let Some(ref end) = params.end_date {
            chrono::DateTime::parse_from_rfc3339(end)
                .map_err(|_| validation_error(&format!("Invalid end_date format: {}", end)))?;
        }

        // Build request
        let date_range = match (&params.start_date, &params.end_date) {
            (Some(start), Some(end)) => Some(DateRange {
                start_date: start.clone(),
                end_date: end.clone(),
            }),
            _ => None,
        };

        let request = SearchRequest {
            query: params.query,
            providers: params.providers,
            projects: params.projects,
            date_range,
            search_type: None,
            page: params.page,
            page_size: params.page_size,
        };

        // Search messages
        let response = self
            .query_service()
            .search_messages(request)
            .await
            .map_err(to_mcp_error)?;

        // Return pretty-printed JSON
        let json = serde_json::to_string_pretty(&response)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    /// Get analytics information for a specific chat session
    #[tool(
        description = "Get analytics information for a specific chat session, including completed analytics results and any pending/running analysis requests"
    )]
    pub async fn get_session_analytics(
        &self,
        params: Parameters<GetSessionAnalyticsParams>,
    ) -> Result<CallToolResult, McpError> {
        let params = params.0;

        // Validate UUID format
        Uuid::parse_str(&params.session_id).map_err(|_| {
            validation_error(&format!(
                "Invalid session_id format: {}. Must be a valid UUID",
                params.session_id
            ))
        })?;

        // Get session analytics
        let response = self
            .query_service()
            .get_session_analytics(&params.session_id)
            .await
            .map_err(|e| {
                let err_msg = e.to_string();
                if err_msg.contains("not found") || err_msg.contains("Session not found") {
                    not_found_error(&params.session_id)
                } else {
                    to_mcp_error(e)
                }
            })?;

        // Manually construct JSON since SessionAnalytics doesn't implement Serialize
        let json = if let Some(analytics) = response {
            let value = serde_json::json!({
                "latest_analytics": analytics.latest_analytics,
                "latest_request": analytics.latest_request,
                "active_request": analytics.active_request,
            });
            serde_json::to_string_pretty(&value)
                .map_err(|e| McpError::internal_error(e.to_string(), None))?
        } else {
            "null".to_string()
        };

        Ok(CallToolResult::success(vec![Content::text(json)]))
    }
}
