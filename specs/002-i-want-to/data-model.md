# Data Model: LLM-Powered Chat Session Retrospection

## Core Entities

### RetrospectionAnalysis
**Purpose**: Represents the output from LLM analysis of chat sessions

**Fields**:
- `id: UUID` - Unique identifier for the analysis
- `session_id: UUID` - Foreign key to chat_sessions table
- `prompt_template_id: String` - ID of the prompt template used
- `analysis_content: String` - Full LLM response text
- `metadata: AnalysisMetadata` - Analysis execution details
- `created_at: DateTime<Utc>` - When analysis was performed
- `updated_at: DateTime<Utc>` - Last modification time
- `status: AnalysisStatus` - Current status of the analysis

**Relationships**:
- Belongs to one ChatSession (existing entity)
- References one PromptTemplate (by ID)
- Has one AnalysisMetadata (embedded)

**State Transitions**:
- `Pending` → `InProgress` → `Completed` | `Failed`
- `Failed` → `Pending` (for retry)

### PromptTemplate
**Purpose**: Stores configurable analysis prompt templates

**Fields**:
- `id: String` - Unique template identifier (slug format)
- `name: String` - Human-readable template name
- `description: String` - Template purpose and usage description
- `template: String` - Prompt template with variable placeholders
- `variables: Vec<PromptVariable>` - Template variable definitions
- `category: String` - Template category (analysis, retrospective, custom)
- `is_default: bool` - Whether this is a built-in template
- `created_at: DateTime<Utc>` - Template creation time
- `modified_at: DateTime<Utc>` - Last modification time

**Validation Rules**:
- `id` must be unique and URL-safe
- `template` must contain required variables
- `template` length must be ≤ 8192 characters
- `variables` must include all placeholders in template

### PromptVariable
**Purpose**: Defines variables within prompt templates

**Fields**:
- `name: String` - Variable name (matches template placeholder)
- `description: String` - Variable purpose and expected content
- `required: bool` - Whether variable must be provided
- `default_value: Option<String>` - Default value if optional

**Validation Rules**:
- `name` must match pattern `[a-zA-Z_][a-zA-Z0-9_]*`
- Required variables cannot have default values

### AnalysisMetadata
**Purpose**: Tracks analysis execution details

**Fields**:
- `llm_service: String` - LLM service used ("gemini-2.5-flash-lite")
- `prompt_tokens: u32` - Input tokens consumed
- `completion_tokens: u32` - Output tokens generated
- `total_tokens: u32` - Total tokens used
- `estimated_cost: Decimal` - Estimated API cost
- `execution_time_ms: u64` - Analysis duration in milliseconds
- `api_response_metadata: Option<String>` - Raw API metadata JSON

### AnalysisRequest
**Purpose**: Represents a user's request to analyze specific chat sessions

**Fields**:
- `id: UUID` - Unique request identifier
- `session_id: UUID` - Target session for analysis
- `prompt_template_id: String` - Template to use for analysis
- `template_variables: HashMap<String, String>` - Variable values
- `status: RequestStatus` - Current request status
- `error_message: Option<String>` - Error details if failed
- `created_at: DateTime<Utc>` - Request creation time
- `started_at: Option<DateTime<Utc>>` - Processing start time
- `completed_at: Option<DateTime<Utc>>` - Processing completion time

**State Transitions**:
- `Queued` → `Processing` → `Completed` | `Failed`
- `Failed` → `Queued` (for retry)

## Database Schema Extensions

### New Tables

```sql
-- Retrospection analyses storage
CREATE TABLE retrospection_analyses (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    prompt_template_id TEXT NOT NULL,
    analysis_content TEXT NOT NULL,
    llm_service TEXT NOT NULL,
    prompt_tokens INTEGER NOT NULL,
    completion_tokens INTEGER NOT NULL,
    total_tokens INTEGER NOT NULL,
    estimated_cost REAL NOT NULL,
    execution_time_ms INTEGER NOT NULL,
    api_response_metadata TEXT,
    status TEXT NOT NULL DEFAULT 'completed',
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (session_id) REFERENCES chat_sessions (id),
    INDEX idx_retrospection_session (session_id),
    INDEX idx_retrospection_created (created_at)
);

-- Prompt templates storage
CREATE TABLE prompt_templates (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT NOT NULL,
    template TEXT NOT NULL,
    category TEXT NOT NULL,
    is_default BOOLEAN NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    modified_at TEXT NOT NULL,
    UNIQUE(name)
);

-- Template variables (normalized)
CREATE TABLE prompt_variables (
    template_id TEXT NOT NULL,
    name TEXT NOT NULL,
    description TEXT NOT NULL,
    required BOOLEAN NOT NULL,
    default_value TEXT,
    PRIMARY KEY (template_id, name),
    FOREIGN KEY (template_id) REFERENCES prompt_templates (id) ON DELETE CASCADE
);

-- Analysis request queue
CREATE TABLE analysis_requests (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    prompt_template_id TEXT NOT NULL,
    template_variables TEXT NOT NULL, -- JSON
    status TEXT NOT NULL DEFAULT 'queued',
    error_message TEXT,
    created_at TEXT NOT NULL,
    started_at TEXT,
    completed_at TEXT,
    FOREIGN KEY (session_id) REFERENCES chat_sessions (id),
    FOREIGN KEY (prompt_template_id) REFERENCES prompt_templates (id),
    INDEX idx_requests_status (status),
    INDEX idx_requests_created (created_at)
);
```

## Enumerations

### AnalysisStatus
```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AnalysisStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
}
```

### RequestStatus
```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RequestStatus {
    Queued,
    Processing,
    Completed,
    Failed,
}
```

## Entity Relationships

```
ChatSession (existing)
    ↓ 1:N
RetrospectionAnalysis
    ↓ N:1
PromptTemplate
    ↓ 1:N
PromptVariable

AnalysisRequest
    ↓ N:1
ChatSession (existing)
    ↓ N:1
PromptTemplate
```

## Data Access Patterns

### Primary Queries
1. **Get analyses for session**: `SELECT * FROM retrospection_analyses WHERE session_id = ?`
2. **List all analyses**: `SELECT * FROM retrospection_analyses ORDER BY created_at DESC`
3. **Get template with variables**: Join prompt_templates and prompt_variables
4. **Active analysis requests**: `SELECT * FROM analysis_requests WHERE status = 'queued'`

### Index Strategy
- `retrospection_analyses.session_id` - Fast lookup by session
- `retrospection_analyses.created_at` - Chronological listing
- `analysis_requests.status` - Queue processing
- `analysis_requests.created_at` - Request ordering

### Data Retention
- Analysis results: Permanent storage (user controls deletion)
- Request queue: Cleanup completed/failed requests after 30 days
- Template history: Keep all versions for audit trail

## Migration Strategy

### Backwards Compatibility
- New tables do not affect existing functionality
- Existing CLI/TUI commands continue to work unchanged
- Database version tracking for future migrations

### Data Seeding
- Insert default prompt templates on first migration
- Set up built-in templates for common analysis types
- Initialize configuration with sensible defaults