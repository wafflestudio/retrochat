# Data Model: LLM Chat History Analysis

## Core Entities

### ChatSession
Represents a complete conversation with an LLM provider.

**Fields**:
- `id`: Unique identifier (UUID)
- `provider`: LLM provider type (ClaudeCode, Gemini, etc.)
- `project_name`: Optional project categorization
- `start_time`: Session start timestamp (ISO8601)
- `end_time`: Session end timestamp (ISO8601)
- `message_count`: Total number of messages in session
- `token_count`: Total tokens used (if available)
- `file_path`: Original file path for reference
- `file_hash`: SHA256 hash for duplicate detection
- `created_at`: Import timestamp
- `updated_at`: Last modification timestamp

**Validation Rules**:
- `start_time` must be valid ISO8601 timestamp
- `end_time` must be after `start_time` if present
- `message_count` must be positive integer
- `token_count` must be non-negative if present
- `provider` must be valid enum value

**State Transitions**:
- Created → Importing → Imported → Analyzed

### Message
Individual message within a chat session.

**Fields**:
- `id`: Unique identifier (UUID)
- `session_id`: Foreign key to ChatSession
- `role`: Message role (User, Assistant, System)
- `content`: Message text content
- `timestamp`: Message timestamp (ISO8601)
- `token_count`: Tokens used for this message (if available)
- `tool_calls`: JSON array of tool calls (if applicable)
- `metadata`: Additional provider-specific metadata (JSON)
- `sequence_number`: Message order within session

**Validation Rules**:
- `content` cannot be empty string
- `role` must be valid enum value
- `timestamp` must be valid ISO8601
- `sequence_number` must be unique within session
- `token_count` must be non-negative if present

**Relationships**:
- Many-to-One with ChatSession
- Ordered by `sequence_number` within session

### Project
Grouping mechanism for related chat sessions.

**Fields**:
- `id`: Unique identifier (UUID)
- `name`: Project display name
- `description`: Optional project description
- `working_directory`: File system path associated with project
- `created_at`: Project creation timestamp
- `updated_at`: Last modification timestamp
- `session_count`: Cached count of associated sessions
- `total_tokens`: Cached sum of tokens across all sessions

**Validation Rules**:
- `name` must be non-empty and unique
- `working_directory` must be valid file path if present
- `session_count` and `total_tokens` must be non-negative

**Relationships**:
- One-to-Many with ChatSession
- Auto-categorization based on file paths and metadata

### UsageAnalysis
Generated insights about user's LLM usage patterns.

**Fields**:
- `id`: Unique identifier (UUID)
- `analysis_type`: Type of analysis (Daily, Weekly, Monthly, Provider, Project)
- `time_period_start`: Analysis period start
- `time_period_end`: Analysis period end
- `provider_filter`: Optional provider filter
- `project_filter`: Optional project filter
- `total_sessions`: Number of sessions analyzed
- `total_messages`: Number of messages analyzed
- `total_tokens`: Total tokens used in period
- `average_session_length`: Average messages per session
- `most_active_day`: Day with highest usage
- `purpose_categories`: JSON object with categorized purposes
- `quality_scores`: JSON object with quality assessments
- `recommendations`: JSON array of actionable recommendations
- `generated_at`: Analysis generation timestamp

**Validation Rules**:
- `time_period_end` must be after `time_period_start`
- Numeric fields must be non-negative
- `analysis_type` must be valid enum value
- `purpose_categories` must be valid JSON object

**Relationships**:
- References ChatSession data through filters
- Cached analysis results for performance

### LlmProvider
Configuration and metadata for supported LLM providers.

**Fields**:
- `id`: Provider identifier (enum)
- `name`: Display name
- `file_patterns`: JSON array of file patterns for detection
- `default_locations`: JSON array of default file locations by OS
- `parser_type`: Parser implementation to use
- `supports_tokens`: Whether provider reports token usage
- `supports_tools`: Whether provider supports tool calls
- `last_updated`: Last time provider support was updated

**Validation Rules**:
- `id` must be unique
- `file_patterns` must be valid glob patterns
- `parser_type` must reference implemented parser

**Relationships**:
- Referenced by ChatSession
- Defines parsing and import behavior

## Database Schema

### SQLite Tables

```sql
-- Chat sessions table
CREATE TABLE chat_sessions (
    id TEXT PRIMARY KEY,
    provider TEXT NOT NULL,
    project_name TEXT,
    start_time TEXT NOT NULL,
    end_time TEXT,
    message_count INTEGER NOT NULL DEFAULT 0,
    token_count INTEGER,
    file_path TEXT NOT NULL,
    file_hash TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (project_name) REFERENCES projects(name),
    UNIQUE(file_hash, file_path)
);

-- Messages table
CREATE TABLE messages (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    role TEXT NOT NULL CHECK (role IN ('User', 'Assistant', 'System')),
    content TEXT NOT NULL CHECK (length(content) > 0),
    timestamp TEXT NOT NULL,
    token_count INTEGER CHECK (token_count >= 0),
    tool_calls TEXT, -- JSON array
    metadata TEXT,   -- JSON object
    sequence_number INTEGER NOT NULL,
    FOREIGN KEY (session_id) REFERENCES chat_sessions(id) ON DELETE CASCADE,
    UNIQUE(session_id, sequence_number)
);

-- Projects table
CREATE TABLE projects (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    description TEXT,
    working_directory TEXT,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    session_count INTEGER NOT NULL DEFAULT 0,
    total_tokens INTEGER NOT NULL DEFAULT 0
);

-- Usage analysis table
CREATE TABLE usage_analyses (
    id TEXT PRIMARY KEY,
    analysis_type TEXT NOT NULL,
    time_period_start TEXT NOT NULL,
    time_period_end TEXT NOT NULL,
    provider_filter TEXT,
    project_filter TEXT,
    total_sessions INTEGER NOT NULL DEFAULT 0,
    total_messages INTEGER NOT NULL DEFAULT 0,
    total_tokens INTEGER NOT NULL DEFAULT 0,
    average_session_length REAL NOT NULL DEFAULT 0,
    most_active_day TEXT,
    purpose_categories TEXT, -- JSON object
    quality_scores TEXT,     -- JSON object
    recommendations TEXT,    -- JSON array
    generated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Provider configuration table
CREATE TABLE llm_providers (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    file_patterns TEXT NOT NULL, -- JSON array
    default_locations TEXT NOT NULL, -- JSON array
    parser_type TEXT NOT NULL,
    supports_tokens BOOLEAN NOT NULL DEFAULT FALSE,
    supports_tools BOOLEAN NOT NULL DEFAULT FALSE,
    last_updated TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
```

### Indexes for Performance

```sql
-- Session queries
CREATE INDEX idx_sessions_provider ON chat_sessions(provider);
CREATE INDEX idx_sessions_project ON chat_sessions(project_name);
CREATE INDEX idx_sessions_start_time ON chat_sessions(start_time);
CREATE INDEX idx_sessions_file_hash ON chat_sessions(file_hash);

-- Message queries
CREATE INDEX idx_messages_session ON messages(session_id);
CREATE INDEX idx_messages_timestamp ON messages(timestamp);
CREATE INDEX idx_messages_role ON messages(role);

-- Analysis queries
CREATE INDEX idx_analysis_type_period ON usage_analyses(analysis_type, time_period_start, time_period_end);

-- Full-text search on message content
CREATE VIRTUAL TABLE messages_fts USING fts5(content, session_id UNINDEXED);
```

## Data Integrity Rules

### Constraints
- No orphaned messages (CASCADE DELETE on session deletion)
- Unique file hash prevents duplicate imports
- Session message count matches actual message count
- Project token totals match sum of session tokens

### Triggers
```sql
-- Update session message count when messages added/removed
CREATE TRIGGER update_session_message_count
    AFTER INSERT ON messages
BEGIN
    UPDATE chat_sessions
    SET message_count = (
        SELECT COUNT(*) FROM messages WHERE session_id = NEW.session_id
    ),
    updated_at = CURRENT_TIMESTAMP
    WHERE id = NEW.session_id;
END;

-- Update project aggregates when sessions change
CREATE TRIGGER update_project_aggregates
    AFTER INSERT ON chat_sessions
BEGIN
    UPDATE projects
    SET session_count = session_count + 1,
        total_tokens = total_tokens + COALESCE(NEW.token_count, 0),
        updated_at = CURRENT_TIMESTAMP
    WHERE name = NEW.project_name;
END;
```

## Performance Considerations

### Query Optimization
- Index on frequently filtered columns (provider, project, timestamp)
- Full-text search index for message content
- Materialized views for complex analytics queries
- Connection pooling for concurrent access

### Memory Management
- Streaming queries for large result sets
- Pagination for UI display
- Lazy loading of message content
- Cached aggregates to avoid expensive calculations

### Scalability Targets
- Support up to 100k chat sessions
- Support up to 10M individual messages
- Query response time <100ms for UI operations
- Import performance <1s per 1MB chat file