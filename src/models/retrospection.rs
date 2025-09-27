use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use uuid::Uuid;

use super::retrospect_request::RetrospectionAnalysisType;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Retrospection {
    pub id: String,
    pub request_id: String, // For API compatibility
    pub insights: String,
    pub reflection: String,
    pub recommendations: String,
    pub metadata: Option<String>, // JSON string
    pub created_at: DateTime<Utc>,
    pub token_usage: Option<u32>,
    pub response_time: Option<Duration>,
}

impl Retrospection {
    pub fn new(
        request_id: String,
        insights: String,
        reflection: String,
        recommendations: String,
        metadata: Option<String>,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            request_id,
            insights,
            reflection,
            recommendations,
            metadata,
            created_at: Utc::now(),
            token_usage: None,
            response_time: None,
        }
    }

    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Result<Self, serde_json::Error> {
        self.metadata = Some(serde_json::to_string(&metadata)?);
        Ok(self)
    }

    pub fn with_token_usage(mut self, token_usage: u32) -> Self {
        self.token_usage = Some(token_usage);
        self
    }

    pub fn with_response_time(mut self, response_time: Duration) -> Self {
        self.response_time = Some(response_time);
        self
    }

    pub fn get_analysis_type(&self) -> Option<RetrospectionAnalysisType> {
        // This could be derived from metadata or stored separately
        // For now, return None as this info is stored in the RetrospectRequest
        None
    }
}
