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
│   │ detected_turns   │               │ messages         │           │
│   │ (rule-based +    │──────────────►│ (unchanged)      │           │
│   │  computed metrics)│  references  └──────────────────┘           │
│   └────────┬─────────┘                       │                      │
│            │                                  │ 1:1                  │
│            │ 1:1                              ▼                      │
│            ▼                          ┌──────────────────┐           │
│   ┌──────────────────┐               │ tool_operations  │           │
│   │ turn_summaries   │               │ (unchanged)      │           │
│   │ (LLM-generated)  │               └──────────────────┘           │
│   └──────────────────┘                                              │
│                                                                      │
└─────────────────────────────────────────────────────────────────────┘
```

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
-- Table: detected_turns
-- Purpose: Rule-based turn boundaries with computed metrics (no LLM required)
-- Lifecycle: Created during import, never modified
-- Data Sources: messages, tool_operations, bash_metadata tables
-- =============================================================================
CREATE TABLE IF NOT EXISTS detected_turns (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    turn_number INTEGER NOT NULL,           -- 0-indexed within session

    -- =========================================================================
    -- BOUNDARIES (references to messages table)
    -- =========================================================================
    start_sequence INTEGER NOT NULL,
    end_sequence INTEGER NOT NULL,
    user_message_id TEXT,                   -- FK to first user message (nullable for turn 0)

    -- =========================================================================
    -- MESSAGE METRICS (computed from messages table)
    -- =========================================================================
    message_count INTEGER NOT NULL DEFAULT 0,
    user_message_count INTEGER NOT NULL DEFAULT 0,
    assistant_message_count INTEGER NOT NULL DEFAULT 0,
    system_message_count INTEGER NOT NULL DEFAULT 0,

    -- By message type
    simple_message_count INTEGER NOT NULL DEFAULT 0,
    tool_request_count INTEGER NOT NULL DEFAULT 0,
    tool_result_count INTEGER NOT NULL DEFAULT 0,
    thinking_count INTEGER NOT NULL DEFAULT 0,
    slash_command_count INTEGER NOT NULL DEFAULT 0,

    -- =========================================================================
    -- TOKEN METRICS (aggregated from messages.token_count)
    -- =========================================================================
    total_token_count INTEGER,              -- Sum of all message tokens
    user_token_count INTEGER,               -- User message tokens
    assistant_token_count INTEGER,          -- Assistant message tokens

    -- =========================================================================
    -- TOOL OPERATION METRICS (from tool_operations table)
    -- =========================================================================
    tool_call_count INTEGER NOT NULL DEFAULT 0,
    tool_success_count INTEGER NOT NULL DEFAULT 0,
    tool_error_count INTEGER NOT NULL DEFAULT 0,

    -- Tool breakdown by name (JSON object)
    tool_usage TEXT,                        -- {"Read": 5, "Write": 3, "Bash": 2, "Edit": 1}

    -- =========================================================================
    -- FILE METRICS (extracted from tool_operations.file_metadata)
    -- =========================================================================
    files_read TEXT,                        -- JSON array: ["src/main.rs", "Cargo.toml"]
    files_written TEXT,                     -- JSON array: ["src/auth.rs"]
    files_modified TEXT,                    -- JSON array: ["src/lib.rs"] (Edit operations)

    unique_files_touched INTEGER DEFAULT 0, -- Count of distinct files across all operations

    -- Line change metrics (aggregated from tool_operations.file_metadata)
    total_lines_added INTEGER DEFAULT 0,
    total_lines_removed INTEGER DEFAULT 0,
    total_lines_changed INTEGER DEFAULT 0,  -- added + removed

    -- =========================================================================
    -- BASH METRICS (from bash_metadata table)
    -- =========================================================================
    bash_command_count INTEGER DEFAULT 0,
    bash_success_count INTEGER DEFAULT 0,
    bash_error_count INTEGER DEFAULT 0,

    -- Commands executed (JSON array)
    commands_executed TEXT,                 -- ["cargo test", "git status", "npm install"]

    -- =========================================================================
    -- CONTENT PREVIEW (for quick display without reading messages)
    -- =========================================================================
    user_message_preview TEXT,              -- First 500 chars of user message
    assistant_message_preview TEXT,         -- First 500 chars of final assistant message

    -- =========================================================================
    -- TIMESTAMPS
    -- =========================================================================
    started_at TEXT NOT NULL,
    ended_at TEXT NOT NULL,
    duration_seconds INTEGER,               -- ended_at - started_at

    created_at TEXT NOT NULL DEFAULT (datetime('now', 'utc')),

    FOREIGN KEY (session_id) REFERENCES chat_sessions(id) ON DELETE CASCADE,
    FOREIGN KEY (user_message_id) REFERENCES messages(id) ON DELETE SET NULL,
    UNIQUE(session_id, turn_number)
);

-- =============================================================================
-- Table: turn_summaries
-- Purpose: LLM-generated summaries for detected turns
-- Lifecycle: Created async by background job, can be regenerated
-- =============================================================================
CREATE TABLE IF NOT EXISTS turn_summaries (
    id TEXT PRIMARY KEY,
    turn_id TEXT NOT NULL,                  -- FK to detected_turns.id

    -- LLM-generated content
    user_intent TEXT NOT NULL,              -- "User wanted to add JWT authentication"
    assistant_action TEXT NOT NULL,         -- "Created auth module with JWT support"
    summary TEXT NOT NULL,                  -- Combined summary sentence

    -- Classification
    turn_type TEXT,                         -- 'task', 'question', 'error_fix', 'clarification', 'discussion'
    complexity_score REAL,                  -- 0.0 - 1.0 (optional)

    -- Generation metadata
    model_used TEXT,
    prompt_version INTEGER NOT NULL DEFAULT 1,
    generated_at TEXT NOT NULL DEFAULT (datetime('now', 'utc')),

    FOREIGN KEY (turn_id) REFERENCES detected_turns(id) ON DELETE CASCADE,
    UNIQUE(turn_id)                         -- 1:1 relationship
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

    -- Aggregated metrics (computed from detected_turns, not LLM)
    total_turns INTEGER NOT NULL DEFAULT 0,
    total_tool_calls INTEGER NOT NULL DEFAULT 0,
    successful_tool_calls INTEGER NOT NULL DEFAULT 0,
    failed_tool_calls INTEGER NOT NULL DEFAULT 0,
    total_lines_changed INTEGER NOT NULL DEFAULT 0,

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
CREATE INDEX IF NOT EXISTS idx_detected_turns_session ON detected_turns(session_id);
CREATE INDEX IF NOT EXISTS idx_detected_turns_started ON detected_turns(started_at);
CREATE INDEX IF NOT EXISTS idx_detected_turns_tool_count ON detected_turns(tool_call_count);
CREATE INDEX IF NOT EXISTS idx_detected_turns_lines_changed ON detected_turns(total_lines_changed);
CREATE INDEX IF NOT EXISTS idx_turn_summaries_turn ON turn_summaries(turn_id);
CREATE INDEX IF NOT EXISTS idx_turn_summaries_type ON turn_summaries(turn_type);
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
    VALUES (NEW.rowid, NEW.summary, NEW.user_intent, NEW.assistant_action, NEW.turn_id);
END;

CREATE TRIGGER IF NOT EXISTS turn_summaries_fts_delete
AFTER DELETE ON turn_summaries BEGIN
    INSERT INTO turn_summaries_fts(turn_summaries_fts, rowid, summary, user_intent, assistant_action, turn_id)
    VALUES ('delete', OLD.rowid, OLD.summary, OLD.user_intent, OLD.assistant_action, OLD.turn_id);
END;

CREATE TRIGGER IF NOT EXISTS turn_summaries_fts_update
AFTER UPDATE ON turn_summaries BEGIN
    INSERT INTO turn_summaries_fts(turn_summaries_fts, rowid, summary, user_intent, assistant_action, turn_id)
    VALUES ('delete', OLD.rowid, OLD.summary, OLD.user_intent, OLD.assistant_action, OLD.turn_id);
    INSERT INTO turn_summaries_fts(rowid, summary, user_intent, assistant_action, turn_id)
    VALUES (NEW.rowid, NEW.summary, NEW.user_intent, NEW.assistant_action, NEW.turn_id);
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

## detected_turns: Data Extraction

The `detected_turns` table contains computed metrics extracted from existing tables without LLM.

### Data Sources

```
┌─────────────────────────────────────────────────────────────────────┐
│                    detected_turns Data Sources                       │
├─────────────────────────────────────────────────────────────────────┤
│                                                                      │
│  messages table                                                      │
│  ──────────────                                                      │
│  → message_count, user/assistant/system counts                       │
│  → message type counts (tool_request, tool_result, thinking, etc)    │
│  → token counts (aggregated from messages.token_count)               │
│  → user_message_preview, assistant_message_preview                   │
│  → timestamps (started_at, ended_at from first/last message)         │
│                                                                      │
│  tool_operations table                                               │
│  ─────────────────────                                               │
│  → tool_call_count, success/error counts                             │
│  → tool_usage breakdown {"Read": 5, "Write": 3}                      │
│  → files_read, files_written, files_modified (from file_metadata)    │
│  → lines_added, lines_removed (from file_metadata)                   │
│                                                                      │
│  bash_metadata table                                                 │
│  ───────────────────                                                 │
│  → bash_command_count, success/error counts                          │
│  → commands_executed                                                 │
│                                                                      │
└─────────────────────────────────────────────────────────────────────┘
```

### Extraction Logic

```rust
pub struct TurnMetricsExtractor;

impl TurnMetricsExtractor {
    /// Extract all computed metrics for a turn from related tables
    pub fn extract(
        messages: &[Message],
        tool_ops: &[ToolOperation],
        bash_meta: &[BashMetadata],
    ) -> TurnMetrics {
        // Message counts by role
        let user_message_count = messages.iter()
            .filter(|m| m.role == MessageRole::User)
            .count();
        let assistant_message_count = messages.iter()
            .filter(|m| m.role == MessageRole::Assistant)
            .count();
        let system_message_count = messages.iter()
            .filter(|m| m.role == MessageRole::System)
            .count();

        // Message counts by type
        let tool_request_count = messages.iter()
            .filter(|m| m.message_type == MessageType::ToolRequest)
            .count();
        let tool_result_count = messages.iter()
            .filter(|m| m.message_type == MessageType::ToolResult)
            .count();
        let thinking_count = messages.iter()
            .filter(|m| m.message_type == MessageType::Thinking)
            .count();

        // Token aggregation
        let total_token_count: u32 = messages.iter()
            .filter_map(|m| m.token_count)
            .sum();
        let user_token_count: u32 = messages.iter()
            .filter(|m| m.role == MessageRole::User)
            .filter_map(|m| m.token_count)
            .sum();

        // Tool usage breakdown
        let mut tool_usage: HashMap<String, u32> = HashMap::new();
        for op in tool_ops {
            *tool_usage.entry(op.tool_name.clone()).or_insert(0) += 1;
        }

        // File categorization
        let files_read: Vec<String> = tool_ops.iter()
            .filter(|op| op.tool_name == "Read")
            .filter_map(|op| op.file_metadata.as_ref())
            .map(|fm| fm.file_path.clone())
            .collect();

        let files_written: Vec<String> = tool_ops.iter()
            .filter(|op| op.tool_name == "Write")
            .filter_map(|op| op.file_metadata.as_ref())
            .map(|fm| fm.file_path.clone())
            .collect();

        let files_modified: Vec<String> = tool_ops.iter()
            .filter(|op| op.tool_name == "Edit")
            .filter_map(|op| op.file_metadata.as_ref())
            .map(|fm| fm.file_path.clone())
            .collect();

        // Line change aggregation
        let total_lines_added: u32 = tool_ops.iter()
            .filter_map(|op| op.file_metadata.as_ref())
            .filter_map(|fm| fm.lines_added)
            .sum();

        let total_lines_removed: u32 = tool_ops.iter()
            .filter_map(|op| op.file_metadata.as_ref())
            .filter_map(|fm| fm.lines_removed)
            .sum();

        // Bash metrics
        let bash_success_count = bash_meta.iter()
            .filter(|b| b.exit_code == Some(0))
            .count();
        let bash_error_count = bash_meta.iter()
            .filter(|b| b.exit_code.map(|c| c != 0).unwrap_or(false))
            .count();

        let commands_executed: Vec<String> = bash_meta.iter()
            .map(|b| b.command.clone())
            .collect();

        // Content previews
        let user_message_preview = messages.iter()
            .find(|m| m.role == MessageRole::User)
            .map(|m| truncate(&m.content, 500));

        let assistant_message_preview = messages.iter()
            .rev()
            .find(|m| m.role == MessageRole::Assistant && m.message_type == MessageType::SimpleMessage)
            .map(|m| truncate(&m.content, 500));

        TurnMetrics {
            message_count: messages.len(),
            user_message_count,
            assistant_message_count,
            system_message_count,
            tool_request_count,
            tool_result_count,
            thinking_count,
            total_token_count,
            user_token_count,
            tool_usage,
            files_read,
            files_written,
            files_modified,
            total_lines_added,
            total_lines_removed,
            bash_command_count: bash_meta.len(),
            bash_success_count,
            bash_error_count,
            commands_executed,
            user_message_preview,
            assistant_message_preview,
            // ... timestamps, duration
        }
    }
}
```

### Comparison: detected_turns vs turn_summaries

| Aspect | detected_turns | turn_summaries |
|--------|---------------|----------------|
| **Source** | Computed from existing tables | LLM-generated |
| **When created** | During import (sync) | Background job (async) |
| **Deterministic** | Yes | No |
| **Cost** | Free (CPU only) | LLM API tokens |
| **Can regenerate** | Yes (idempotent) | Yes (may differ) |
| **Example fields** | `tool_call_count`, `files_modified`, `total_lines_added` | `user_intent`, `summary`, `turn_type` |

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
        // Also fetch detected_turns for metrics
        let detected_turns = self.detected_turn_repo
            .get_by_session(session_id)
            .await?;

        // Build prompt from turn summaries
        let prompt = self.build_session_prompt_from_turns(turn_summaries);

        // Call LLM
        let llm_response = self.llm_client.generate(&prompt).await?;

        // Combine LLM output with computed metrics from detected_turns
        Ok(SessionSummary {
            title: llm_response.title,
            summary: llm_response.summary,
            primary_goal: llm_response.primary_goal,
            outcome: llm_response.outcome,
            key_decisions: llm_response.key_decisions,
            technologies_used: llm_response.technologies_used,
            files_affected: self.aggregate_files(&detected_turns),
            // Computed metrics (not from LLM)
            total_turns: detected_turns.len(),
            total_tool_calls: detected_turns.iter().map(|t| t.tool_call_count).sum(),
            successful_tool_calls: detected_turns.iter().map(|t| t.tool_success_count).sum(),
            failed_tool_calls: detected_turns.iter().map(|t| t.tool_error_count).sum(),
            total_lines_changed: detected_turns.iter().map(|t| t.total_lines_changed).sum(),
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
Total tool calls: {tool_call_count}
Files touched: {unique_files_count}
Duration: {duration_minutes} minutes
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
  "technologies_used": ["tech1", "tech2", "library1"]
}
```

---

## Implementation Phases

### Phase 1: Schema + Turn Detection

**Goal**: Add new tables and implement rule-based turn detection with computed metrics

**Files to create/modify**:
```
crates/retrochat-core/
├── migrations/
│   └── 018_add_hierarchical_storage.sql    # New migration
├── src/
│   ├── models/
│   │   ├── mod.rs                          # Export new models
│   │   ├── detected_turn.rs                # New model with all metrics
│   │   ├── turn_summary.rs                 # New model
│   │   └── session_summary.rs              # New model
│   ├── database/
│   │   ├── mod.rs                          # Export new repos
│   │   ├── detected_turn_repo.rs           # New repo
│   │   ├── turn_summary_repo.rs            # New repo
│   │   └── session_summary_repo.rs         # New repo
│   └── services/
│       ├── mod.rs                          # Export new services
│       ├── turn_detection.rs               # Turn boundary detection
│       └── turn_metrics.rs                 # Metrics extraction from messages/tool_ops
```

**Turn Detection Algorithm**:
```rust
pub struct TurnDetector;

impl TurnDetector {
    /// Detect turn boundaries from a list of messages
    ///
    /// Rules:
    /// - User message (SimpleMessage or SlashCommand) starts a new turn
    /// - All other messages belong to the current turn
    /// - If session starts with non-User message, create turn_number = 0
    pub fn detect_turns(
        messages: &[Message],
        tool_ops: &[ToolOperation],
        bash_meta: &[BashMetadata],
    ) -> Vec<DetectedTurn> {
        let mut turns = Vec::new();
        let mut current_turn: Option<TurnBuilder> = None;

        for msg in messages {
            let is_user_turn_start = msg.role == MessageRole::User
                && matches!(
                    msg.message_type,
                    MessageType::SimpleMessage | MessageType::SlashCommand
                );

            if is_user_turn_start {
                // Finalize previous turn with metrics
                if let Some(builder) = current_turn.take() {
                    let turn = builder.build_with_metrics(tool_ops, bash_meta);
                    turns.push(turn);
                }
                // Start new turn
                current_turn = Some(TurnBuilder::new_user_turn(turns.len() as u32, msg));
            } else {
                // Add to current turn or create turn 0
                if current_turn.is_none() {
                    current_turn = Some(TurnBuilder::new_system_turn());
                }
                current_turn.as_mut().unwrap().add_message(msg);
            }
        }

        // Finalize last turn
        if let Some(builder) = current_turn {
            let turn = builder.build_with_metrics(tool_ops, bash_meta);
            turns.push(turn);
        }

        turns
    }
}
```

**Deliverables**:
- [ ] Migration file added
- [ ] DetectedTurn model with all metric fields
- [ ] TurnMetricsExtractor for computing metrics
- [ ] TurnSummary model + repository
- [ ] SessionSummary model + repository
- [ ] TurnDetector service with unit tests
- [ ] `sqlx prepare` updated

---

### Phase 2: Integration with Import Pipeline

**Goal**: Automatically detect turns and compute metrics when sessions are imported

**Modify**:
```
crates/retrochat-core/src/services/
├── import.rs                    # Hook turn detection after message import
└── turn_detection.rs            # Add batch processing
```

**Integration Point**:
```rust
// In ImportService::import_session()
impl ImportService {
    pub async fn import_session(&self, ...) -> Result<ChatSession> {
        // ... existing import logic ...

        // After messages and tool_operations are saved
        let messages = self.message_repo.get_by_session(&session.id).await?;
        let tool_ops = self.tool_op_repo.get_by_session(&session.id).await?;
        let bash_meta = self.bash_meta_repo.get_by_session(&session.id).await?;

        // Detect turns with computed metrics
        let turns = TurnDetector::detect_turns(&messages, &tool_ops, &bash_meta);
        for turn in turns {
            self.detected_turn_repo.create(&turn).await?;
        }

        // Note: Summaries are NOT generated here (async job)

        Ok(session)
    }
}
```

**Deliverables**:
- [ ] Import pipeline hooks turn detection
- [ ] Metrics computed for all new imports
- [ ] Integration tests

---

### Phase 3: Backfill Existing Sessions

**Goal**: Generate detected_turns with metrics for all existing sessions

**Create**:
```
crates/retrochat-core/src/services/
└── backfill.rs                  # One-time backfill logic
```

**CLI Command**:
```bash
# Backfill all sessions
cargo run -p retrochat-cli -- backfill turns

# Backfill specific session
cargo run -p retrochat-cli -- backfill turns --session <SESSION_ID>

# Check backfill status
cargo run -p retrochat-cli -- backfill status
```

**Deliverables**:
- [ ] BackfillService with progress tracking
- [ ] CLI command for manual trigger
- [ ] Idempotent (safe to run multiple times)

---

### Phase 4: Turn Summarization Service

**Goal**: LLM-based turn summarization

**Create**:
```
crates/retrochat-core/src/services/
└── turn_summarizer.rs           # LLM summarization logic
```

**Prompt Design**:
```
Summarize this conversation turn between a user and an AI coding assistant.

<turn_metadata>
Tool calls: {tool_call_count} ({tool_success_count} succeeded, {tool_error_count} failed)
Files touched: {files_list}
Lines changed: +{lines_added} -{lines_removed}
</turn_metadata>

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
  "turn_type": "task|question|error_fix|clarification|discussion"
}
```

**Batching Strategy**:
```rust
impl TurnSummarizer {
    /// Summarize multiple turns in one LLM call (max 5-10)
    pub async fn summarize_batch(&self, turns: &[DetectedTurn]) -> Result<Vec<TurnSummary>> {
        // Build prompt with multiple turns
        // Parse multiple JSON responses
        // Return summaries matched to turn IDs
    }
}
```

**Deliverables**:
- [ ] TurnSummarizer service
- [ ] Prompt templates (include metrics context)
- [ ] Batch processing (5-10 turns per call)
- [ ] Rate limiting + retry logic
- [ ] Unit tests with mock LLM

---

### Phase 5: Session Summarization Service

**Goal**: Generate session-level summaries from turn summaries

**Create**:
```
crates/retrochat-core/src/services/
└── session_summarizer.rs        # Session-level summarization
```

**Implementation**:
- Primary: Generate from turn_summaries (small input, cheap)
- Fallback: Generate from raw messages if no turn summaries exist
- Metrics: Aggregate from detected_turns (not LLM)

**Deliverables**:
- [ ] SessionSummarizer service
- [ ] Primary path: from turn summaries
- [ ] Fallback path: from raw messages
- [ ] Metric aggregation from detected_turns
- [ ] Unit tests

---

### Phase 6: Background Job System

**Goal**: Async processing of summarization

**Options**:
1. **Simple**: Process on CLI command (`cargo cli summarize`)
2. **Background**: Spawn tokio task during import
3. **Queue**: Persist pending work, process incrementally

**Recommended**: Option 1 (Simple) for MVP

**CLI Commands**:
```bash
# Summarize pending turns
cargo run -p retrochat-cli -- summarize turns [--session <ID>] [--all]

# Summarize pending sessions
cargo run -p retrochat-cli -- summarize sessions [--session <ID>] [--all]

# Check summarization status
cargo run -p retrochat-cli -- summarize status
```

**Status Query**:
```sql
-- Turns pending summarization
SELECT
    dt.session_id,
    COUNT(*) as pending_turns
FROM detected_turns dt
LEFT JOIN turn_summaries ts ON dt.id = ts.turn_id
WHERE ts.id IS NULL
GROUP BY dt.session_id;

-- Sessions pending summarization
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

### Phase 7: Search Integration

**Goal**: Use summaries for improved search

**Modify**:
```
crates/retrochat-core/src/services/
└── search.rs                    # Add summary-based search
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

### Phase 8: MCP Server Updates

**Goal**: Expose hierarchical data via MCP

**New/Updated Tools**:
```
list_sessions      → Include has_summary flag, turn_count
get_session_detail → Include session_summary + turns with metrics
search_messages    → Add scope parameter (sessions/turns/messages)
NEW: list_turns    → List turns for a session with metrics
NEW: get_turn      → Get turn with summary + metrics + raw messages
```

**Deliverables**:
- [ ] MCP tools updated
- [ ] New turn-level tools
- [ ] Documentation

---

## Query Examples

### Metrics-based Queries (No LLM Required)

```sql
-- Find turns with most file changes
SELECT
    dt.id,
    dt.session_id,
    dt.total_lines_changed,
    dt.files_modified,
    dt.tool_call_count
FROM detected_turns dt
ORDER BY dt.total_lines_changed DESC
LIMIT 10;

-- Aggregate metrics per session
SELECT
    session_id,
    COUNT(*) as turn_count,
    SUM(tool_call_count) as total_tools,
    SUM(bash_command_count) as total_bash,
    SUM(total_lines_added) as lines_added,
    SUM(total_lines_removed) as lines_removed
FROM detected_turns
GROUP BY session_id;

-- Find error-heavy turns
SELECT dt.*, ts.summary
FROM detected_turns dt
LEFT JOIN turn_summaries ts ON dt.id = ts.turn_id
WHERE dt.tool_error_count > 3
   OR dt.bash_error_count > 2;

-- Most used tools across all turns
SELECT
    json_each.value as tool_name,
    COUNT(*) as usage_count
FROM detected_turns, json_each(detected_turns.tool_usage)
GROUP BY json_each.value
ORDER BY usage_count DESC;
```

### Summary-based Queries (After LLM Processing)

```sql
-- Search turns by intent
SELECT dt.*, ts.*
FROM detected_turns dt
JOIN turn_summaries ts ON dt.id = ts.turn_id
JOIN turn_summaries_fts fts ON ts.rowid = fts.rowid
WHERE turn_summaries_fts MATCH 'authentication OR auth';

-- Find all error_fix turns
SELECT dt.*, ts.summary
FROM detected_turns dt
JOIN turn_summaries ts ON dt.id = ts.turn_id
WHERE ts.turn_type = 'error_fix';

-- Session search
SELECT ss.*, cs.project_name
FROM session_summaries ss
JOIN chat_sessions cs ON ss.session_id = cs.id
JOIN session_summaries_fts fts ON ss.rowid = fts.rowid
WHERE session_summaries_fts MATCH 'JWT authentication';
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
   DROP TABLE IF EXISTS detected_turns;
   ```

---

## Success Criteria

- [ ] All existing tests pass
- [ ] New sessions automatically get turns detected with metrics
- [ ] Existing sessions can be backfilled
- [ ] Metrics queries work without LLM (tool counts, file changes, etc.)
- [ ] Search on summaries returns relevant results
- [ ] MCP tools work with hierarchical data
- [ ] No degradation in import performance
