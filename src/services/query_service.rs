use crate::database::{ChatSessionRepository, DatabaseManager};
use crate::models::{ChatSession, Message};
use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

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
        let db_manager = Arc::new(DatabaseManager::new("retrochat.db").await.unwrap());
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
        let mut sessions = Vec::new();

        for session in paginated_sessions {
            // Get first message preview
            let first_message_preview = message_repo
                .get_by_session(&session.id)
                .await
                .ok()
                .and_then(|messages| {
                    messages.first().map(|msg| {
                        let preview = if msg.content.len() > 100 {
                            // Find a safe character boundary
                            let mut end = 97;
                            while end > 0 && !msg.content.is_char_boundary(end) {
                                end -= 1;
                            }
                            format!("{}...", &msg.content[..end])
                        } else {
                            msg.content.clone()
                        };
                        preview
                    })
                })
                .unwrap_or_else(|| "No messages available".to_string());

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

        // Search for messages using FTS with filters
        let messages = message_repo
            .search_content_with_filters(
                &request.query,
                None,      // session_id filter
                None,      // role filter
                Some(100), // limit
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
            let content_snippet = if message.content.len() > 200 {
                // Find a safe character boundary
                let mut end = 197;
                while end > 0 && !message.content.is_char_boundary(end) {
                    end -= 1;
                }
                format!("...{}...", &message.content[..end])
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
}

impl Default for QueryService {
    fn default() -> Self {
        // Use a blocking approach for Default implementation
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async { Self::new().await })
        })
    }
}
