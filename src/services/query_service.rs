use crate::database::{
    AnalyticsRepository, AnalyticsRequestRepository, ChatSessionRepository, DatabaseManager,
};
use crate::models::{Analytics, AnalyticsRequest, ChatSession, Message, OperationStatus};
use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::Arc;
use uuid::Uuid;

/// Represents a message or group of related messages for display purposes
#[derive(Debug, Clone)]
pub enum MessageGroup {
    /// A single standalone message
    Single(Message),
    /// A tool use message paired with its corresponding tool result message
    ToolPair {
        tool_use_message: Message,
        tool_result_message: Message,
    },
}

impl MessageGroup {
    /// Pairs tool_use and tool_result messages that appear in separate messages.
    ///
    /// This function groups consecutive messages where:
    /// - Message N has tool_uses
    /// - Message N+1 has tool_results with matching tool_use_id
    ///
    /// Messages with both tool_uses and tool_results are kept as Single (already paired).
    pub fn pair_tool_messages(messages: Vec<Message>) -> Vec<Self> {
        let mut groups = Vec::new();
        let mut i = 0;

        while i < messages.len() {
            let current = &messages[i];

            // Check if this message has tool_uses but no tool_results (potential pair start)
            let has_tool_uses = current
                .tool_uses
                .as_ref()
                .is_some_and(|uses| !uses.is_empty());
            let has_tool_results = current
                .tool_results
                .as_ref()
                .is_some_and(|results| !results.is_empty());

            if has_tool_uses && !has_tool_results && i + 1 < messages.len() {
                // Check if next message has matching tool_results
                let next = &messages[i + 1];

                if let (Some(tool_uses), Some(tool_results)) =
                    (&current.tool_uses, &next.tool_results)
                {
                    // Check if the next message has ONLY tool_results (no tool_uses)
                    let next_has_tool_uses =
                        next.tool_uses.as_ref().is_some_and(|uses| !uses.is_empty());

                    if !next_has_tool_uses {
                        // Collect all tool_use IDs from current message
                        let tool_use_ids: HashSet<&str> =
                            tool_uses.iter().map(|u| u.id.as_str()).collect();

                        // Check if any tool_result matches any tool_use
                        let has_matching_result = tool_results
                            .iter()
                            .any(|r| tool_use_ids.contains(r.tool_use_id.as_str()));

                        if has_matching_result {
                            // Create a ToolPair and skip the next message
                            groups.push(MessageGroup::ToolPair {
                                tool_use_message: current.clone(),
                                tool_result_message: next.clone(),
                            });
                            i += 2; // Skip both messages
                            continue;
                        }
                    }
                }
            }

            // Not a pair, add as single
            groups.push(MessageGroup::Single(current.clone()));
            i += 1;
        }

        groups
    }

    /// Returns all messages contained in this group (for iteration)
    pub fn messages(&self) -> Vec<&Message> {
        match self {
            MessageGroup::Single(msg) => vec![msg],
            MessageGroup::ToolPair {
                tool_use_message,
                tool_result_message,
            } => vec![tool_use_message, tool_result_message],
        }
    }

    /// Returns true if this is a tool pair
    pub fn is_tool_pair(&self) -> bool {
        matches!(self, MessageGroup::ToolPair { .. })
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SessionsQueryRequest {
    pub page: Option<i32>,
    pub page_size: Option<i32>,
    pub sort_by: Option<String>,
    pub sort_order: Option<String>,
    pub filters: Option<SessionFilters>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SessionFilters {
    pub provider: Option<String>,
    pub project: Option<String>,
    pub date_range: Option<DateRange>,
    pub min_messages: Option<i32>,
    pub max_messages: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DateRange {
    pub start_date: String,
    pub end_date: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SessionsQueryResponse {
    pub sessions: Vec<SessionSummary>,
    pub total_count: i32,
    pub page: i32,
    pub page_size: i32,
    pub total_pages: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SessionSummary {
    pub session_id: String,
    pub provider: String,
    pub project: Option<String>,
    pub start_time: String,
    pub end_time: String,
    pub message_count: i32,
    pub total_tokens: Option<i32>,
    pub first_message_preview: String,
    pub has_analytics: bool,
    pub analytics_status: Option<OperationStatus>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SessionDetailRequest {
    pub session_id: String,
    pub include_content: Option<bool>,
    pub message_limit: Option<i32>,
    pub message_offset: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SessionDetailResponse {
    pub session: ChatSession,
    pub messages: Vec<Message>,
    pub total_message_count: i32,
    pub has_more_messages: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchRequest {
    pub query: String,
    pub providers: Option<Vec<String>>,
    pub projects: Option<Vec<String>>,
    pub date_range: Option<DateRange>,
    pub search_type: Option<String>,
    pub page: Option<i32>,
    pub page_size: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchResponse {
    pub results: Vec<SearchResult>,
    pub total_count: i32,
    pub page: i32,
    pub page_size: i32,
    pub search_duration_ms: i32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SearchResult {
    pub session_id: String,
    pub message_id: String,
    pub provider: String,
    pub project: Option<String>,
    pub timestamp: String,
    pub content_snippet: String,
    pub message_role: String,
    pub relevance_score: f64,
}

pub struct QueryService {
    db_manager: Arc<DatabaseManager>,
}

impl QueryService {
    pub async fn new() -> Self {
        // For backward compatibility, use a shared database instance
        let db_path = crate::database::config::get_default_db_path().unwrap();
        let db_manager = Arc::new(DatabaseManager::new(&db_path).await.unwrap());
        Self { db_manager }
    }

    pub fn with_database(db_manager: Arc<DatabaseManager>) -> Self {
        Self { db_manager }
    }

    pub async fn query_sessions(
        &self,
        request: SessionsQueryRequest,
    ) -> Result<SessionsQueryResponse> {
        let page = request.page.unwrap_or(1);
        let page_size = request.page_size.unwrap_or(20);
        let sort_by = request.sort_by.unwrap_or_else(|| "start_time".to_string());
        let sort_order = request.sort_order.unwrap_or_else(|| "desc".to_string());

        let session_repo = ChatSessionRepository::new(&self.db_manager);

        // Get all sessions first (we'll implement pagination later)
        let all_sessions = session_repo.get_all().await?;

        // Apply filters if specified
        let filtered_sessions: Vec<ChatSession> = if let Some(filters) = &request.filters {
            all_sessions
                .into_iter()
                .filter(|session| {
                    // Filter by provider
                    if let Some(ref provider_filter) = filters.provider {
                        if session.provider.to_string() != *provider_filter {
                            return false;
                        }
                    }

                    // Filter by project
                    if let Some(ref project_filter) = filters.project {
                        if session.project_name.as_deref() != Some(project_filter) {
                            return false;
                        }
                    }

                    // Filter by message count
                    if let Some(min_messages) = filters.min_messages {
                        if (session.message_count as i32) < min_messages {
                            return false;
                        }
                    }

                    if let Some(max_messages) = filters.max_messages {
                        if (session.message_count as i32) > max_messages {
                            return false;
                        }
                    }

                    // Implement date range filtering
                    if let Some(ref date_range) = filters.date_range {
                        let session_start = session.start_time;
                        let start_date = date_range.start_date.parse::<DateTime<Utc>>().ok();
                        let end_date = date_range.end_date.parse::<DateTime<Utc>>().ok();

                        if let Some(start) = start_date {
                            if session_start < start {
                                return false;
                            }
                        }

                        if let Some(end) = end_date {
                            if session_start > end {
                                return false;
                            }
                        }
                    }

                    true
                })
                .collect()
        } else {
            all_sessions
        };

        // Sort sessions
        let mut sorted_sessions = filtered_sessions;
        sorted_sessions.sort_by(|a, b| {
            let ordering = match sort_by.as_str() {
                "message_count" => a.message_count.cmp(&b.message_count),
                "provider" => a.provider.to_string().cmp(&b.provider.to_string()),
                "project" => a.project_name.cmp(&b.project_name),
                _ => a.start_time.cmp(&b.start_time), // default to start_time
            };

            if sort_order == "desc" {
                ordering.reverse()
            } else {
                ordering
            }
        });

        let total_count = sorted_sessions.len() as i32;
        let total_pages = (total_count + page_size - 1) / page_size;

        // Apply pagination
        let offset = ((page - 1) * page_size) as usize;
        let limit = page_size as usize;
        let paginated_sessions: Vec<ChatSession> = sorted_sessions
            .into_iter()
            .skip(offset)
            .take(limit)
            .collect();

        // Convert to SessionSummary format with actual first message preview
        let message_repo = crate::database::MessageRepository::new(&self.db_manager);
        let analytics_request_repo = AnalyticsRequestRepository::new(self.db_manager.clone());
        let mut sessions = Vec::new();

        for session in paginated_sessions {
            // Get first message preview
            let first_message_preview = message_repo
                .get_by_session(&session.id)
                .await
                .ok()
                .and_then(|messages| {
                    messages.first().map(|msg| {
                        let preview = if msg.content.chars().count() > 100 {
                            let truncated: String = msg.content.chars().take(97).collect();
                            format!("{truncated}...")
                        } else {
                            msg.content.clone()
                        };
                        preview
                    })
                })
                .unwrap_or_else(|| "No messages available".to_string());

            // Check for analytics requests for this session
            let (has_analytics, analytics_status) = analytics_request_repo
                .find_by_session_id(&session.id.to_string())
                .await
                .ok()
                .and_then(|requests| {
                    if requests.is_empty() {
                        None
                    } else {
                        // Get the most recent request status
                        let latest_status = requests.first().map(|r| r.status.clone());
                        Some((true, latest_status))
                    }
                })
                .unwrap_or((false, None));

            sessions.push(SessionSummary {
                session_id: session.id.to_string(),
                provider: session.provider.to_string(),
                project: session.project_name,
                start_time: session.start_time.to_rfc3339(),
                end_time: session
                    .end_time
                    .map(|t| t.to_rfc3339())
                    .unwrap_or_else(|| session.start_time.to_rfc3339()),
                message_count: session.message_count as i32,
                total_tokens: session.token_count.map(|t| t as i32),
                first_message_preview,
                has_analytics,
                analytics_status,
            });
        }

        Ok(SessionsQueryResponse {
            sessions,
            total_count,
            page,
            page_size,
            total_pages,
        })
    }

    pub async fn get_session_detail(
        &self,
        request: SessionDetailRequest,
    ) -> Result<SessionDetailResponse> {
        // Parse session ID from request
        let session_id = Uuid::parse_str(&request.session_id)
            .map_err(|e| anyhow::anyhow!("Invalid session ID: {e}"))?;

        // Get session from database
        let session_repo = ChatSessionRepository::new(&self.db_manager);
        let session = session_repo
            .get_by_id(&session_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Session not found: {session_id}"))?;

        // Get messages for this session
        let message_repo = crate::database::MessageRepository::new(&self.db_manager);
        let messages = message_repo.get_by_session(&session_id).await?;

        Ok(SessionDetailResponse {
            session,
            total_message_count: messages.len() as i32,
            messages,
            has_more_messages: false, // For now, we load all messages
        })
    }

    pub async fn search_messages(&self, request: SearchRequest) -> Result<SearchResponse> {
        let start_time = std::time::Instant::now();

        // Use the message repository's search functionality
        let message_repo = crate::database::MessageRepository::new(&self.db_manager);
        let session_repo = ChatSessionRepository::new(&self.db_manager);

        // Parse date range if provided
        let (start_datetime, end_datetime) = if let Some(ref date_range) = request.date_range {
            let start = DateTime::parse_from_rfc3339(&date_range.start_date)
                .map(|dt| dt.with_timezone(&Utc))
                .ok();
            let end = DateTime::parse_from_rfc3339(&date_range.end_date)
                .map(|dt| dt.with_timezone(&Utc))
                .ok();
            (start, end)
        } else {
            (None, None)
        };

        // Search for messages using FTS with filters
        let messages = message_repo
            .search_content_with_time_filters(
                &request.query,
                None,           // session_id filter
                None,           // role filter
                start_datetime, // from timestamp
                end_datetime,   // to timestamp
                Some(100),      // limit
            )
            .await?;

        // Convert to search results
        let mut results = Vec::new();

        for message in messages {
            // Get session info for context
            let session = session_repo
                .get_by_id(&message.session_id)
                .await
                .ok()
                .flatten();

            // Create content snippet
            let content_snippet = if message.content.chars().count() > 200 {
                let truncated: String = message.content.chars().take(197).collect();
                format!("...{truncated}...")
            } else {
                message.content.clone()
            };

            results.push(SearchResult {
                session_id: message.session_id.to_string(),
                message_id: message.id.to_string(),
                provider: session
                    .as_ref()
                    .map(|s| s.provider.to_string())
                    .unwrap_or_else(|| "unknown".to_string()),
                project: session.and_then(|s| s.project_name),
                timestamp: message.timestamp.to_rfc3339(),
                content_snippet,
                message_role: message.role.to_string(),
                relevance_score: 0.8, // FTS doesn't provide relevance scores, use default
            });
        }

        // Sort by relevance score (descending) for consistent ordering
        results.sort_by(|a, b| {
            b.relevance_score
                .partial_cmp(&a.relevance_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Apply pagination
        let page = request.page.unwrap_or(1);
        let page_size = request.page_size.unwrap_or(20);
        let total_count = results.len() as i32;

        let start_idx = ((page - 1) * page_size) as usize;
        let end_idx = (start_idx + page_size as usize).min(results.len());

        let paginated_results = if start_idx < results.len() {
            results[start_idx..end_idx].to_vec()
        } else {
            Vec::new()
        };

        let search_duration_ms = start_time.elapsed().as_millis() as i32;

        Ok(SearchResponse {
            total_count,
            results: paginated_results,
            page,
            page_size,
            search_duration_ms,
        })
    }

    /// Get analytics information for a session
    /// Returns both the latest completed analytics and any pending/running requests
    pub async fn get_session_analytics(
        &self,
        session_id: &str,
    ) -> Result<Option<SessionAnalytics>> {
        let analytics_request_repo = AnalyticsRequestRepository::new(self.db_manager.clone());
        let analytics_repo = AnalyticsRepository::new(&self.db_manager);

        // Get all analytics requests for this session
        let requests = analytics_request_repo
            .find_by_session_id(session_id)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to fetch analytics requests: {e}"))?;

        if requests.is_empty() {
            return Ok(None);
        }

        // Find the most recent completed request
        let completed_request = requests
            .iter()
            .find(|r| r.status == OperationStatus::Completed);

        // Get the analytics result if available
        let analytics = if let Some(request) = completed_request {
            analytics_repo
                .get_analytics_by_request_id(&request.id)
                .await
                .ok()
                .flatten()
        } else {
            None
        };

        // Find any active (pending/running) requests
        let active_request = requests
            .iter()
            .find(|r| r.status == OperationStatus::Pending || r.status == OperationStatus::Running)
            .cloned();

        Ok(Some(SessionAnalytics {
            latest_analytics: analytics,
            latest_request: requests.first().cloned(),
            active_request,
        }))
    }
}

/// Analytics information for a session
#[derive(Debug, Clone)]
pub struct SessionAnalytics {
    /// The latest completed analytics result (if any)
    pub latest_analytics: Option<Analytics>,
    /// The most recent analytics request (regardless of status)
    pub latest_request: Option<AnalyticsRequest>,
    /// Any currently active (pending/running) request
    pub active_request: Option<AnalyticsRequest>,
}

impl Default for QueryService {
    fn default() -> Self {
        // Use a blocking approach for Default implementation
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async { Self::new().await })
        })
    }
}
