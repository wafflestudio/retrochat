-- Migration: 018_add_hierarchical_storage.sql
-- Purpose: Add hierarchical summarization layer (detected_turns, turn_summaries, session_summaries)

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
