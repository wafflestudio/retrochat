# Hierarchical Storage Migration Plan

## Overview

Add hierarchical summarization layer on top of existing message storage without removing any existing tables.

```
┌─────────────────────────────────────────────────────────────────────┐
│                     Target Architecture                              │
├─────────────────────────────────────────────────────────────────────┤
│                                                                      │
│   NEW LAYER (summaries)              EXISTING LAYER (raw data)       │
│   ─────────────────────              ─────────────────────────       │
│                                                                      │
│   ┌──────────────────┐               ┌──────────────────┐           │
│   │ session_summaries│               │ chat_sessions    │           │
│   │ (LLM-generated)  │───────────────│ (unchanged)      │           │
│   └────────┬─────────┘               └──────────────────┘           │
│            │                                  │                      │
│            │ 1:N                              │ 1:N                  │
│            ▼                                  ▼                      │
│   ┌──────────────────┐               ┌──────────────────┐           │
│   │ turn_summaries   │──────────────►│ messages         │           │
│   │ (LLM-generated)  │  references   │ (unchanged)      │           │
│   │                  │  via sequence │                  │           │
│   └──────────────────┘               └──────────────────┘           │
│                                               │                      │
│                                               │ 1:1                  │
│                                               ▼                      │
│                                       ┌──────────────────┐           │
│                                       │ tool_operations  │           │
│                                       │ (unchanged)      │           │
│                                       └──────────────────┘           │
│                                                                      │
└─────────────────────────────────────────────────────────────────────┘
```

## Key Design Decision: No Precomputed Metrics Table

This plan intentionally **removes the `detected_turns` table** from the original design.

### Why No Precomputed Metrics?

1. **Core problem is semantic search**: Turn summaries solve this directly. Computed metrics are secondary.

2. **Metrics can be computed on-demand**: Given a turn's `start_sequence` and `end_sequence`, we can always query:
   ```sql
   SELECT
       COUNT(*) as message_count,
       SUM(CASE WHEN message_type = 'tool_request' THEN 1 ELSE 0 END) as tool_requests
   FROM messages
   WHERE session_id = ? AND sequence_number BETWEEN ? AND ?
   ```

3. **Single source of truth**: Messages remain canonical. No risk of metrics becoming stale.

4. **Less code to maintain**: Fewer tables, fewer migrations, simpler mental model.

5. **YAGNI**: If metrics performance becomes an issue later, add materialized views or caching.

---

## Turn Detection Rules

### What is a Turn?

A turn starts with a **User message** and includes all following messages until the next User message.

```
Messages:                              Turn Assignment:
─────────────────────────────────────────────────────────
1. User: "Add auth"                    ┐
2. Assistant: "I'll help..."           │
3. Assistant: [ToolRequest: Read]      │  Turn 1
4. System: [ToolResult: file content]  │
5. Assistant: [ToolRequest: Write]     │
6. System: [ToolResult: success]       │
7. Assistant: "Done!"                  ┘
─────────────────────────────────────────────────────────
8. User: "Add tests"                   ┐
9. Assistant: "Sure..."                │  Turn 2
10. Assistant: [ToolRequest: Write]    │
11. System: [ToolResult: success]      ┘
─────────────────────────────────────────────────────────
```

### Turn Boundary Rules

| Message Type | Starts New Turn? | Notes |
|-------------|------------------|-------|
| `User` + `SimpleMessage` | Yes | Primary turn boundary |
| `User` + `SlashCommand` | Yes | User-initiated command |
| `Assistant` + `SimpleMessage` | No | Part of current turn |
| `Assistant` + `ToolRequest` | No | Part of current turn |
| `Assistant` + `Thinking` | No | Part of current turn |
| `System` + `ToolResult` | No | Response to assistant |
| `System` + `SimpleMessage` | No | System notification |

### Edge Cases

1. **Session starts with Assistant message**: Create turn with `turn_number = 0` (system-initiated)
2. **Multiple User messages in sequence**: Each starts a new turn (even if assistant didn't respond)
3. **Empty assistant response**: Still a valid turn (user asked, got no answer)

---

## Schema Changes

### New Tables

```sql
-- Migration: 018_add_hierarchical_storage.sql

-- =============================================================================
-- Table: turn_summaries
-- Purpose: LLM-generated summaries with direct references to messages
-- Lifecycle: Created async by background job, can be regenerated
-- =============================================================================
CREATE TABLE IF NOT EXISTS turn_summaries (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    turn_number INTEGER NOT NULL,           -- 0-indexed within session

    -- =========================================================================
    -- MESSAGE BOUNDARIES (references to messages table)
    -- =========================================================================
    start_sequence INTEGER NOT NULL,        -- First message sequence in turn
    end_sequence INTEGER NOT NULL,          -- Last message sequence in turn
    user_message_id TEXT,                   -- FK to first user message (nullable for turn 0)

    -- =========================================================================
    -- LLM-GENERATED CONTENT
    -- =========================================================================
    user_intent TEXT NOT NULL,              -- "User wanted to add JWT authentication"
    assistant_action TEXT NOT NULL,         -- "Created auth module with JWT support"
    summary TEXT NOT NULL,                  -- Combined summary sentence

    -- Classification
    turn_type TEXT,                         -- 'task', 'question', 'error_fix', 'clarification', 'discussion'
    complexity_score REAL,                  -- 0.0 - 1.0 (optional)

    -- Extracted entities (JSON arrays)
    key_topics TEXT,                        -- ["authentication", "JWT", "middleware"]
    decisions_made TEXT,                    -- ["Used RS256 over HS256", "Added refresh tokens"]
    code_concepts TEXT,                     -- ["error handling", "async/await", "middleware pattern"]

    -- =========================================================================
    -- CACHED TIMESTAMPS (derived from messages, cached for convenience)
    -- =========================================================================
    started_at TEXT NOT NULL,
    ended_at TEXT NOT NULL,

    -- =========================================================================
    -- GENERATION METADATA
    -- =========================================================================
    model_used TEXT,
    prompt_version INTEGER NOT NULL DEFAULT 1,
    generated_at TEXT NOT NULL DEFAULT (datetime('now', 'utc')),

    FOREIGN KEY (session_id) REFERENCES chat_sessions(id) ON DELETE CASCADE,
    FOREIGN KEY (user_message_id) REFERENCES messages(id) ON DELETE SET NULL,
    UNIQUE(session_id, turn_number)
);

-- =============================================================================
-- Table: session_summaries
-- Purpose: LLM-generated session-level summaries
-- Lifecycle: Created after turn summaries exist, can be regenerated
-- Input: Aggregated from turn_summaries (NOT raw messages)
-- =============================================================================
CREATE TABLE IF NOT EXISTS session_summaries (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,

    -- LLM-generated content
    title TEXT NOT NULL,                    -- "JWT Authentication Implementation"
    summary TEXT NOT NULL,                  -- 100-200 word overview
    primary_goal TEXT,                      -- Main user objective
    outcome TEXT,                           -- 'completed', 'partial', 'abandoned', 'ongoing'

    -- Extracted entities (JSON arrays)
    key_decisions TEXT,                     -- ["Used JWT over sessions", "RS256 signing"]
    technologies_used TEXT,                 -- ["JWT", "bcrypt", "axum"]
    files_affected TEXT,                    -- ["src/auth.rs", "src/middleware.rs"]

    -- Generation metadata
    model_used TEXT,
    prompt_version INTEGER NOT NULL DEFAULT 1,
    generated_at TEXT NOT NULL DEFAULT (datetime('now', 'utc')),

    FOREIGN KEY (session_id) REFERENCES chat_sessions(id) ON DELETE CASCADE,
    UNIQUE(session_id)                      -- 1:1 relationship
);

-- =============================================================================
-- Indexes
-- =============================================================================
CREATE INDEX IF NOT EXISTS idx_turn_summaries_session ON turn_summaries(session_id);
CREATE INDEX IF NOT EXISTS idx_turn_summaries_type ON turn_summaries(turn_type);
CREATE INDEX IF NOT EXISTS idx_turn_summaries_started ON turn_summaries(started_at);
CREATE INDEX IF NOT EXISTS idx_session_summaries_session ON session_summaries(session_id);
CREATE INDEX IF NOT EXISTS idx_session_summaries_outcome ON session_summaries(outcome);

-- =============================================================================
-- Full-Text Search
-- =============================================================================
CREATE VIRTUAL TABLE IF NOT EXISTS turn_summaries_fts USING fts5(
    summary,
    user_intent,
    assistant_action,
    turn_id UNINDEXED,
    content='turn_summaries',
    content_rowid='rowid'
);

CREATE VIRTUAL TABLE IF NOT EXISTS session_summaries_fts USING fts5(
    title,
    summary,
    primary_goal,
    session_id UNINDEXED,
    content='session_summaries',
    content_rowid='rowid'
);

-- FTS triggers for turn_summaries
CREATE TRIGGER IF NOT EXISTS turn_summaries_fts_insert
AFTER INSERT ON turn_summaries BEGIN
    INSERT INTO turn_summaries_fts(rowid, summary, user_intent, assistant_action, turn_id)
    VALUES (NEW.rowid, NEW.summary, NEW.user_intent, NEW.assistant_action, NEW.id);
END;

CREATE TRIGGER IF NOT EXISTS turn_summaries_fts_delete
AFTER DELETE ON turn_summaries BEGIN
    INSERT INTO turn_summaries_fts(turn_summaries_fts, rowid, summary, user_intent, assistant_action, turn_id)
    VALUES ('delete', OLD.rowid, OLD.summary, OLD.user_intent, OLD.assistant_action, OLD.id);
END;

CREATE TRIGGER IF NOT EXISTS turn_summaries_fts_update
AFTER UPDATE ON turn_summaries BEGIN
    INSERT INTO turn_summaries_fts(turn_summaries_fts, rowid, summary, user_intent, assistant_action, turn_id)
    VALUES ('delete', OLD.rowid, OLD.summary, OLD.user_intent, OLD.assistant_action, OLD.id);
    INSERT INTO turn_summaries_fts(rowid, summary, user_intent, assistant_action, turn_id)
    VALUES (NEW.rowid, NEW.summary, NEW.user_intent, NEW.assistant_action, NEW.id);
END;

-- FTS triggers for session_summaries
CREATE TRIGGER IF NOT EXISTS session_summaries_fts_insert
AFTER INSERT ON session_summaries BEGIN
    INSERT INTO session_summaries_fts(rowid, title, summary, primary_goal, session_id)
    VALUES (NEW.rowid, NEW.title, NEW.summary, NEW.primary_goal, NEW.session_id);
END;

CREATE TRIGGER IF NOT EXISTS session_summaries_fts_delete
AFTER DELETE ON session_summaries BEGIN
    INSERT INTO session_summaries_fts(session_summaries_fts, rowid, title, summary, primary_goal, session_id)
    VALUES ('delete', OLD.rowid, OLD.title, OLD.summary, OLD.primary_goal, OLD.session_id);
END;

CREATE TRIGGER IF NOT EXISTS session_summaries_fts_update
AFTER UPDATE ON session_summaries BEGIN
    INSERT INTO session_summaries_fts(session_summaries_fts, rowid, title, summary, primary_goal, session_id)
    VALUES ('delete', OLD.rowid, OLD.title, OLD.summary, OLD.primary_goal, OLD.session_id);
    INSERT INTO session_summaries_fts(rowid, title, summary, primary_goal, session_id)
    VALUES (NEW.rowid, NEW.title, NEW.summary, NEW.primary_goal, NEW.session_id);
END;
```

### Existing Tables (UNCHANGED)

- `chat_sessions` - No changes
- `messages` - No changes
- `messages_fts` - No changes
- `tool_operations` - No changes
- `bash_metadata` - No changes
- `analytics` - No changes
- `analytics_request` - No changes

---

## On-Demand Metrics Computation

Instead of storing precomputed metrics in a `detected_turns` table, compute them on-demand from messages.

### Metrics Helper Functions

```rust
/// Compute metrics for a turn by querying messages within the turn boundaries
pub struct TurnMetrics {
    // Message counts
    pub message_count: usize,
    pub user_message_count: usize,
    pub assistant_message_count: usize,
    pub system_message_count: usize,

    // By message type
    pub tool_request_count: usize,
    pub tool_result_count: usize,
    pub thinking_count: usize,

    // Token metrics
    pub total_token_count: Option<u32>,
    pub user_token_count: Option<u32>,
    pub assistant_token_count: Option<u32>,

    // Tool operation metrics (from tool_operations table)
    pub tool_call_count: usize,
    pub tool_success_count: usize,
    pub tool_error_count: usize,
    pub tool_usage: HashMap<String, u32>,  // {"Read": 5, "Write": 3}

    // File metrics
    pub files_read: Vec<String>,
    pub files_written: Vec<String>,
    pub files_modified: Vec<String>,
    pub total_lines_added: u32,
    pub total_lines_removed: u32,

    // Bash metrics
    pub bash_command_count: usize,
    pub bash_success_count: usize,
    pub bash_error_count: usize,
    pub commands_executed: Vec<String>,
}

impl TurnMetrics {
    /// Compute metrics from messages within a turn's boundaries
    pub async fn compute(
        session_id: &Uuid,
        start_sequence: u32,
        end_sequence: u32,
        message_repo: &MessageRepository,
        tool_op_repo: &ToolOperationRepository,
    ) -> Result<Self> {
        // Query messages in this turn's range
        let messages = message_repo
            .get_by_sequence_range(session_id, start_sequence, end_sequence)
            .await?;

        // Query tool operations for messages in this range
        let message_ids: Vec<_> = messages.iter().map(|m| m.id).collect();
        let tool_ops = tool_op_repo
            .get_by_message_ids(&message_ids)
            .await?;

        // Compute all metrics from the data
        Self::from_data(&messages, &tool_ops)
    }

    fn from_data(messages: &[Message], tool_ops: &[ToolOperation]) -> Self {
        // ... compute all metrics
    }
}
```

### SQL Queries for Metrics

```sql
-- Get message counts for a turn
SELECT
    COUNT(*) as message_count,
    SUM(CASE WHEN role = 'User' THEN 1 ELSE 0 END) as user_count,
    SUM(CASE WHEN role = 'Assistant' THEN 1 ELSE 0 END) as assistant_count,
    SUM(CASE WHEN role = 'System' THEN 1 ELSE 0 END) as system_count,
    SUM(CASE WHEN message_type = 'tool_request' THEN 1 ELSE 0 END) as tool_request_count,
    SUM(CASE WHEN message_type = 'tool_result' THEN 1 ELSE 0 END) as tool_result_count,
    SUM(token_count) as total_tokens
FROM messages
WHERE session_id = ?
  AND sequence_number BETWEEN ? AND ?;

-- Get tool operation summary for messages in a turn
SELECT
    tool_name,
    COUNT(*) as usage_count,
    SUM(CASE WHEN is_error = 0 THEN 1 ELSE 0 END) as success_count,
    SUM(CASE WHEN is_error = 1 THEN 1 ELSE 0 END) as error_count
FROM tool_operations
WHERE message_id IN (
    SELECT id FROM messages
    WHERE session_id = ?
      AND sequence_number BETWEEN ? AND ?
)
GROUP BY tool_name;

-- Get file changes for messages in a turn
SELECT
    SUM(json_extract(file_metadata, '$.lines_added')) as lines_added,
    SUM(json_extract(file_metadata, '$.lines_removed')) as lines_removed
FROM tool_operations
WHERE message_id IN (
    SELECT id FROM messages
    WHERE session_id = ?
      AND sequence_number BETWEEN ? AND ?
)
AND file_metadata IS NOT NULL;
```

---

## Session Summary Generation

### Primary Approach: From Turn Summaries

Session summaries are generated from aggregated turn summaries, NOT from raw messages.

```
┌─────────────────────────────────────────────────────────────────────┐
│                 Session Summary Generation Flow                      │
├─────────────────────────────────────────────────────────────────────┤
│                                                                      │
│  turn_summaries (all turns for session)                              │
│       │                                                              │
│       ▼                                                              │
│  ┌─────────────────────────────────────────────────────────────┐    │
│  │ LLM Input (~100-500 tokens):                                │    │
│  │                                                              │    │
│  │ Turn 1: User wanted auth, Claude created auth module         │    │
│  │ Turn 2: User requested tests, Claude added unit tests        │    │
│  │ Turn 3: Fixed token validation bug                           │    │
│  │ Turn 4: Added refresh token endpoint                         │    │
│  │ ...                                                          │    │
│  └─────────────────────────────────────────────────────────────┘    │
│       │                                                              │
│       ▼                                                              │
│  session_summary                                                     │
│                                                                      │
└─────────────────────────────────────────────────────────────────────┘
```

**Benefits:**
- Much smaller input (~100-500 tokens vs 5,000-50,000 tokens for raw messages)
- Cheaper LLM cost (~$0.001 vs ~$0.01-0.10)
- Faster processing
- Already distilled information
- Never exceeds context window

### Fallback: From Raw Messages (Truncated)

If turn summaries don't exist, fall back to summarizing raw messages directly.

```rust
impl SessionSummarizer {
    pub async fn summarize(&self, session_id: &Uuid) -> Result<SessionSummary> {
        // Primary: from turn summaries
        let turn_summaries = self.turn_summary_repo
            .get_by_session(session_id)
            .await?;

        if !turn_summaries.is_empty() {
            return self.summarize_from_turns(session_id, &turn_summaries).await;
        }

        // Fallback: from raw messages (truncated to fit context)
        let messages = self.message_repo
            .get_by_session(session_id)
            .await?;

        self.summarize_from_messages(session_id, &messages).await
    }

    async fn summarize_from_turns(
        &self,
        session_id: &Uuid,
        turn_summaries: &[TurnSummary],
    ) -> Result<SessionSummary> {
        // Compute metrics on-demand for session-level aggregation
        let mut total_tool_calls = 0;
        let mut total_lines_changed = 0;

        for turn in turn_summaries {
            let metrics = TurnMetrics::compute(
                session_id,
                turn.start_sequence,
                turn.end_sequence,
                &self.message_repo,
                &self.tool_op_repo,
            ).await?;
            total_tool_calls += metrics.tool_call_count;
            total_lines_changed += metrics.total_lines_added + metrics.total_lines_removed;
        }

        // Build prompt from turn summaries
        let prompt = self.build_session_prompt_from_turns(turn_summaries);

        // Call LLM
        let llm_response = self.llm_client.generate(&prompt).await?;

        Ok(SessionSummary {
            title: llm_response.title,
            summary: llm_response.summary,
            primary_goal: llm_response.primary_goal,
            outcome: llm_response.outcome,
            key_decisions: llm_response.key_decisions,
            technologies_used: llm_response.technologies_used,
            files_affected: llm_response.files_affected,
            // ...
        })
    }
}
```

### Prompt Design for Session Summary

```
Summarize this coding session based on the turn summaries below.

<session_metadata>
Total turns: {total_turns}
</session_metadata>

<turns>
Turn 1: {turn_1_summary}
Turn 2: {turn_2_summary}
...
Turn N: {turn_n_summary}
</turns>

Respond in JSON format:
{
  "title": "Short descriptive title (5-8 words)",
  "summary": "Overview of what was accomplished (50-100 words)",
  "primary_goal": "The main user objective (10-15 words)",
  "outcome": "completed|partial|abandoned|ongoing",
  "key_decisions": ["Important choice 1", "Important choice 2"],
  "technologies_used": ["tech1", "tech2", "library1"],
  "files_affected": ["file1.rs", "file2.rs"]
}
```

---

## Implementation Phases

### Phase 1: Schema + Models

**Goal**: Add new tables and create Rust models

**Files to create/modify**:
```
crates/retrochat-core/
├── migrations/
│   └── 018_add_hierarchical_storage.sql    # New migration
├── src/
│   ├── models/
│   │   ├── mod.rs                          # Export new models
│   │   ├── turn_summary.rs                 # New model
│   │   └── session_summary.rs              # New model
│   ├── database/
│   │   ├── mod.rs                          # Export new repos
│   │   ├── turn_summary_repo.rs            # New repo
│   │   └── session_summary_repo.rs         # New repo
```

**Deliverables**:
- [ ] Migration file added
- [ ] TurnSummary model with message boundary references
- [ ] SessionSummary model
- [ ] TurnSummaryRepository with CRUD + FTS search
- [ ] SessionSummaryRepository with CRUD + FTS search
- [ ] `sqlx prepare` updated

---

### Phase 2: Turn Detection Service

**Goal**: Implement rule-based turn boundary detection (without storing a separate table)

**Create**:
```
crates/retrochat-core/src/services/
├── turn_detection.rs               # Turn boundary detection
└── turn_metrics.rs                 # On-demand metrics computation
```

**Turn Detection Algorithm**:
```rust
/// Represents detected turn boundaries (not persisted, used for summarization)
pub struct DetectedTurnBoundary {
    pub turn_number: u32,
    pub start_sequence: u32,
    pub end_sequence: u32,
    pub user_message_id: Option<Uuid>,
    pub started_at: DateTime<Utc>,
    pub ended_at: DateTime<Utc>,
}

pub struct TurnDetector;

impl TurnDetector {
    /// Detect turn boundaries from a list of messages
    ///
    /// Rules:
    /// - User message (SimpleMessage or SlashCommand) starts a new turn
    /// - All other messages belong to the current turn
    /// - If session starts with non-User message, create turn_number = 0
    pub fn detect_boundaries(messages: &[Message]) -> Vec<DetectedTurnBoundary> {
        let mut boundaries = Vec::new();
        let mut current_start: Option<(u32, u32, Option<Uuid>, DateTime<Utc>)> = None;

        for msg in messages {
            let is_user_turn_start = msg.role == MessageRole::User
                && matches!(
                    msg.message_type,
                    MessageType::SimpleMessage | MessageType::SlashCommand
                );

            if is_user_turn_start {
                // Finalize previous turn
                if let Some((turn_num, start_seq, user_msg_id, started_at)) = current_start.take() {
                    let prev_msg = &messages[messages.iter()
                        .position(|m| m.sequence_number == msg.sequence_number)
                        .unwrap() - 1];
                    boundaries.push(DetectedTurnBoundary {
                        turn_number: turn_num,
                        start_sequence: start_seq,
                        end_sequence: prev_msg.sequence_number,
                        user_message_id: user_msg_id,
                        started_at,
                        ended_at: prev_msg.timestamp,
                    });
                }
                // Start new turn
                current_start = Some((
                    boundaries.len() as u32,
                    msg.sequence_number,
                    Some(msg.id),
                    msg.timestamp,
                ));
            } else if current_start.is_none() {
                // Session starts with non-User message (turn 0)
                current_start = Some((0, msg.sequence_number, None, msg.timestamp));
            }
        }

        // Finalize last turn
        if let Some((turn_num, start_seq, user_msg_id, started_at)) = current_start {
            if let Some(last_msg) = messages.last() {
                boundaries.push(DetectedTurnBoundary {
                    turn_number: turn_num,
                    start_sequence: start_seq,
                    end_sequence: last_msg.sequence_number,
                    user_message_id: user_msg_id,
                    started_at,
                    ended_at: last_msg.timestamp,
                });
            }
        }

        boundaries
    }
}
```

**Deliverables**:
- [ ] TurnDetector service with boundary detection
- [ ] TurnMetrics for on-demand computation
- [ ] Unit tests for turn detection logic
- [ ] Unit tests for metrics computation

---

### Phase 3: Turn Summarization Service

**Goal**: LLM-based turn summarization

**Create**:
```
crates/retrochat-core/src/services/
└── turn_summarizer.rs              # LLM summarization logic
```

**Prompt Design**:
```
Summarize this conversation turn between a user and an AI coding assistant.

<turn_content>
User: {user_message}

Assistant responses:
{assistant_messages}

Tools used:
{tool_calls_summary}
</turn_content>

Respond in JSON format:
{
  "user_intent": "What the user wanted (10-20 words)",
  "assistant_action": "What the assistant did (10-20 words)",
  "summary": "One sentence combining both (20-30 words)",
  "turn_type": "task|question|error_fix|clarification|discussion",
  "key_topics": ["topic1", "topic2"],
  "decisions_made": ["decision1", "decision2"],
  "code_concepts": ["concept1", "concept2"]
}
```

**Batching Strategy**:
```rust
impl TurnSummarizer {
    /// Summarize multiple turns in one LLM call (max 5-10)
    pub async fn summarize_batch(
        &self,
        session_id: &Uuid,
        boundaries: &[DetectedTurnBoundary],
    ) -> Result<Vec<TurnSummary>> {
        // Fetch messages for all turns in batch
        // Build combined prompt
        // Parse multiple JSON responses
        // Return summaries with correct turn boundaries
    }
}
```

**Deliverables**:
- [ ] TurnSummarizer service
- [ ] Prompt templates
- [ ] Batch processing (5-10 turns per call)
- [ ] Rate limiting + retry logic
- [ ] Unit tests with mock LLM

---

### Phase 4: Session Summarization Service

**Goal**: Generate session-level summaries from turn summaries

**Create**:
```
crates/retrochat-core/src/services/
└── session_summarizer.rs           # Session-level summarization
```

**Implementation**:
- Primary: Generate from turn_summaries (small input, cheap)
- Fallback: Generate from raw messages if no turn summaries exist
- Metrics: Compute on-demand when needed

**Deliverables**:
- [ ] SessionSummarizer service
- [ ] Primary path: from turn summaries
- [ ] Fallback path: from raw messages
- [ ] Unit tests

---

### Phase 5: CLI Commands

**Goal**: CLI commands for summarization

**CLI Commands**:
```bash
# Summarize turns for a session
cargo run -p retrochat-cli -- summarize turns [--session <ID>] [--all]

# Summarize sessions (from turn summaries)
cargo run -p retrochat-cli -- summarize sessions [--session <ID>] [--all]

# Check summarization status
cargo run -p retrochat-cli -- summarize status
```

**Status Query**:
```sql
-- Sessions without turn summaries
SELECT
    cs.id,
    cs.project_name,
    cs.message_count,
    (SELECT COUNT(*) FROM turn_summaries ts WHERE ts.session_id = cs.id) as turn_count
FROM chat_sessions cs
WHERE NOT EXISTS (
    SELECT 1 FROM turn_summaries ts WHERE ts.session_id = cs.id
);

-- Sessions without session summary
SELECT
    cs.id,
    cs.project_name,
    ss.id IS NULL as needs_summary
FROM chat_sessions cs
LEFT JOIN session_summaries ss ON cs.id = ss.session_id
WHERE ss.id IS NULL;
```

**Deliverables**:
- [ ] SummarizationService orchestrator
- [ ] CLI commands
- [ ] Progress tracking
- [ ] Resume capability (process incrementally)

---

### Phase 6: Search Integration

**Goal**: Use summaries for improved search

**Modify**:
```
crates/retrochat-core/src/services/
└── search.rs                       # Add summary-based search
```

**Search Layers**:
```rust
pub enum SearchScope {
    Sessions,     // Search session_summaries_fts
    Turns,        // Search turn_summaries_fts
    Messages,     // Search messages_fts (existing)
    All,          // Search all layers, dedupe by session
}

impl SearchService {
    pub async fn search(&self, query: &str, scope: SearchScope) -> Result<SearchResults> {
        match scope {
            SearchScope::Sessions => self.search_sessions(query).await,
            SearchScope::Turns => self.search_turns(query).await,
            SearchScope::Messages => self.search_messages(query).await,  // existing
            SearchScope::All => self.search_all(query).await,
        }
    }
}
```

**Deliverables**:
- [ ] Multi-layer search
- [ ] Search result ranking
- [ ] CLI `search` command updated
- [ ] MCP tool updated

---

### Phase 7: MCP Server Updates

**Goal**: Expose hierarchical data via MCP

**New/Updated Tools**:
```
list_sessions      → Include has_summary flag, turn_count
get_session_detail → Include session_summary + turns
search_messages    → Add scope parameter (sessions/turns/messages)
NEW: list_turns    → List turns for a session
NEW: get_turn      → Get turn with summary + messages
```

**Deliverables**:
- [ ] MCP tools updated
- [ ] New turn-level tools
- [ ] Documentation

---

## Query Examples

### Metrics Queries (Computed On-Demand)

```sql
-- Get metrics for a specific turn
SELECT
    COUNT(*) as message_count,
    SUM(CASE WHEN role = 'User' THEN 1 ELSE 0 END) as user_count,
    SUM(CASE WHEN message_type = 'tool_request' THEN 1 ELSE 0 END) as tool_requests,
    SUM(token_count) as total_tokens
FROM messages
WHERE session_id = ?
  AND sequence_number BETWEEN ? AND ?;

-- Get all turn boundaries for a session (for display)
SELECT
    ts.turn_number,
    ts.start_sequence,
    ts.end_sequence,
    ts.summary,
    ts.turn_type,
    (SELECT COUNT(*) FROM messages m
     WHERE m.session_id = ts.session_id
       AND m.sequence_number BETWEEN ts.start_sequence AND ts.end_sequence) as message_count
FROM turn_summaries ts
WHERE ts.session_id = ?
ORDER BY ts.turn_number;
```

### Summary-based Queries

```sql
-- Search turns by intent
SELECT ts.*, cs.project_name
FROM turn_summaries ts
JOIN chat_sessions cs ON ts.session_id = cs.id
JOIN turn_summaries_fts fts ON ts.rowid = fts.rowid
WHERE turn_summaries_fts MATCH 'authentication OR auth';

-- Find all error_fix turns
SELECT ts.summary, ts.session_id, ts.turn_number
FROM turn_summaries ts
WHERE ts.turn_type = 'error_fix';

-- Session search
SELECT ss.*, cs.project_name
FROM session_summaries ss
JOIN chat_sessions cs ON ss.session_id = cs.id
JOIN session_summaries_fts fts ON ss.rowid = fts.rowid
WHERE session_summaries_fts MATCH 'JWT authentication';

-- Sessions with most turns
SELECT
    cs.id,
    cs.project_name,
    ss.title,
    COUNT(ts.id) as turn_count
FROM chat_sessions cs
LEFT JOIN session_summaries ss ON cs.id = ss.session_id
LEFT JOIN turn_summaries ts ON cs.id = ts.session_id
GROUP BY cs.id
ORDER BY turn_count DESC
LIMIT 10;
```

---

## Rollback Plan

If issues arise:
1. New tables are additive (don't break existing functionality)
2. Existing `messages` table unchanged (can always fall back)
3. Drop new tables if needed:
   ```sql
   DROP TABLE IF EXISTS turn_summaries_fts;
   DROP TABLE IF EXISTS session_summaries_fts;
   DROP TABLE IF EXISTS turn_summaries;
   DROP TABLE IF EXISTS session_summaries;
   ```

---

## Success Criteria

- [ ] All existing tests pass
- [ ] Turn summaries can be generated for sessions
- [ ] Session summaries can be generated from turn summaries
- [ ] Metrics computed on-demand work correctly
- [ ] Search on summaries returns relevant results
- [ ] MCP tools work with hierarchical data
- [ ] No degradation in import performance
