# Data Model: Retrospection Feature

## Core Entities

### RetrospectRequest
Represents a single retrospection analysis request for one chat session.

**Fields**:
- `id` (String, Primary Key): Unique identifier for the retrospection request
- `session_id` (String): Single chat session ID being analyzed
- `analysis_type` (AnalysisType): Type of analysis requested
- `status` (OperationStatus): Current status of the analysis
- `started_at` (DateTime<Utc>): When the analysis was initiated
- `completed_at` (Option<DateTime<Utc>>): When the analysis finished
- `created_by` (Option<String>): User or system identifier
- `error_message` (Option<String>): Error details if analysis failed
- `custom_prompt` (Option<String>): Custom analysis prompt if using Custom analysis type

**Relationships**:
- One-to-one with ChatSession via session_id
- One-to-one with Retrospection (result) when completed

**Validation Rules**:
- `id` must be unique and non-empty
- `session_id` must reference a valid ChatSession
- `completed_at` can only be set when status is Completed, Failed, or Cancelled
- `custom_prompt` is required when analysis_type is Custom

**State Transitions**:
```
Pending → Running → Completed
                 → Failed
                 → Cancelled
```

### Retrospection
Contains the actual analysis output from Google AI for a retrospection request.

**Fields**:
- `id` (String, Primary Key): Unique identifier for the retrospection result
- `retrospect_request_id` (String, Foreign Key): Reference to the original request
- `response_text` (String): Raw LLM response containing analysis
- `token_usage` (Option<u32>): Number of tokens consumed by the analysis
- `response_time_ms` (Option<u64>): Time taken for LLM to respond
- `model_used` (Option<String>): Google AI model used for analysis
- `created_at` (DateTime<Utc>): When the result was generated
- `metadata` (Option<String>): JSON string with additional metadata

**Relationships**:
- One-to-one with RetrospectRequest via retrospect_request_id

**Validation Rules**:
- `retrospect_request_id` must reference valid RetrospectRequest
- `response_text` must be non-empty for completed results
- `token_usage` must be positive if provided
- `metadata` must be valid JSON if provided

### AnalysisType (Enum)
Defines the type of retrospection analysis to perform.

**Variants**:
- `UserInteractionAnalysis`: Analyze user communication patterns and effectiveness
- `CollaborationInsights`: Identify collaboration strengths and weaknesses
- `QuestionQuality`: Evaluate clarity and effectiveness of user questions
- `TaskBreakdown`: Analyze user's task decomposition skills
- `FollowUpPatterns`: Examine user's follow-up and iteration patterns
- `Custom(String)`: User-defined analysis prompt

### OperationStatus (Enum)
Tracks the current state of background operations.

**Variants**:
- `Pending`: Operation queued but not started
- `Running`: Operation currently executing
- `Completed`: Operation finished successfully
- `Failed`: Operation encountered unrecoverable error
- `Cancelled`: Operation was cancelled by user or system

## Database Schema (SQLite)

### retrospect_requests table
```sql
CREATE TABLE retrospect_requests (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    analysis_type TEXT NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('pending', 'running', 'completed', 'failed', 'cancelled')),
    started_at TEXT NOT NULL DEFAULT (datetime('now', 'utc')),
    completed_at TEXT,
    created_by TEXT,
    error_message TEXT,
    custom_prompt TEXT,
    FOREIGN KEY (session_id) REFERENCES chat_sessions(id) ON DELETE CASCADE,
    FOREIGN KEY (created_by) REFERENCES users(id) ON DELETE SET NULL
);

CREATE INDEX idx_retrospect_requests_status ON retrospect_requests(status);
CREATE INDEX idx_retrospect_requests_session_id ON retrospect_requests(session_id);
CREATE INDEX idx_retrospect_requests_created_by ON retrospect_requests(created_by);
CREATE INDEX idx_retrospect_requests_started_at ON retrospect_requests(started_at);
```

### retrospections table
```sql
CREATE TABLE retrospections (
    id TEXT PRIMARY KEY,
    retrospect_request_id TEXT NOT NULL,
    response_text TEXT NOT NULL,
    token_usage INTEGER,
    response_time_ms INTEGER,
    model_used TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now', 'utc')),
    metadata TEXT, -- JSON
    FOREIGN KEY (retrospect_request_id) REFERENCES retrospect_requests(id) ON DELETE CASCADE
);

CREATE INDEX idx_retrospections_request_id ON retrospections(retrospect_request_id);
CREATE INDEX idx_retrospections_created_at ON retrospections(created_at);
```

## API Request/Response Models

### GoogleAI Request Models
```rust
#[derive(Debug, Serialize, Deserialize)]
pub struct GenerateContentRequest {
    pub contents: Vec<Content>,
    pub generation_config: Option<GenerationConfig>,
    pub safety_settings: Option<Vec<SafetySetting>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Content {
    pub parts: Vec<Part>,
    pub role: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Part {
    Text { text: String },
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GenerationConfig {
    pub temperature: Option<f32>,
    pub max_output_tokens: Option<u32>,
    pub top_p: Option<f32>,
    pub top_k: Option<u32>,
}
```

### GoogleAI Response Models
```rust
#[derive(Debug, Serialize, Deserialize)]
pub struct GenerateContentResponse {
    pub candidates: Vec<Candidate>,
    pub usage_metadata: Option<UsageMetadata>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Candidate {
    pub content: Content,
    pub finish_reason: Option<String>,
    pub safety_ratings: Option<Vec<SafetyRating>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UsageMetadata {
    pub prompt_token_count: Option<u32>,
    pub candidates_token_count: Option<u32>,
    pub total_token_count: Option<u32>,
}
```

## Domain Service Models

### RetrospectionRequest
Input model for initiating retrospection analysis for a single session.

```rust
#[derive(Debug, Clone)]
pub struct RetrospectionRequest {
    pub session_id: String,
    pub analysis_type: AnalysisType,
    pub custom_prompt: Option<String>,
    pub user_id: Option<String>,
}
```

### RetrospectionProgress
Model for tracking analysis progress.

```rust
#[derive(Debug, Clone)]
pub struct RetrospectionProgress {
    pub request_id: String,
    pub session_id: String,
    pub status: OperationStatus,
    pub message: String,
    pub error: Option<String>,
    pub started_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
```

### AnalysisResult
Processed result from retrospection analysis.

```rust
#[derive(Debug, Clone)]
pub struct AnalysisResult {
    pub session_id: String,
    pub analysis_text: String,
    pub insights: Vec<String>,
    pub token_usage: Option<u32>,
    pub processing_time: Duration,
}
```

## Data Flow

### Analysis Request Flow
1. User initiates retrospection for a single session via CLI or TUI
2. RetrospectionRequest created with session_id and analysis type
3. RetrospectRequest entity created in database with Pending status
4. Background task spawned to process analysis
5. Request status updated to Running
6. Chat session analysis:
   - Chat data formatted for Google AI
   - API request sent with retry logic
   - Response received and validated
   - Retrospection entity created with LLM response
7. Request status updated to Completed/Failed

### Batch Processing Flow (Multiple Sessions)
1. User initiates analysis for multiple sessions
2. Multiple RetrospectRequest entities created (one per session)
3. Background task manager processes requests concurrently
4. Each request follows individual analysis flow
5. Overall progress tracked across all requests

### Progress Tracking Flow
1. Background operation manager tracks individual request progress
2. Progress updates sent via channels to UI components
3. Database updated with request status changes
4. UI widgets display real-time progress for active requests
5. Users can cancel individual requests or all active requests

### Result Retrieval Flow
1. User requests retrospection results via CLI or TUI
2. Query retrospections table joined with retrospect_requests by session_id
3. Format results for display in appropriate interface
4. Support filtering by analysis type, status, and date range

## Storage Considerations

### Data Retention
- RetrospectRequest records kept for 90 days after completion
- Retrospection records kept for 1 year for user reference
- Failed requests cleaned up after 7 days
- Configurable retention policies via application settings

### Performance Optimization
- Index on frequently queried fields (status, session_id, created_at)
- Lazy loading of large response_text fields
- Pagination for result listing operations
- Background cleanup of old records

### Data Integrity
- Foreign key constraints ensure referential integrity
- Check constraints validate enum values
- Triggers maintain updated_at timestamps
- Transaction boundaries for multi-table operations