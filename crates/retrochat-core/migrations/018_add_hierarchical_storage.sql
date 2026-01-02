-- Migration: 018_add_hierarchical_storage.sql
-- Description: Add hierarchical summarization layer with turn and session summaries

-- =============================================================================
-- Table: turn_summaries
-- Purpose: LLM-generated summaries with direct references to messages
-- Lifecycle: Created async by background job, can be regenerated
-- =============================================================================
CREATE TABLE IF NOT EXISTS turn_summaries (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    turn_number INTEGER NOT NULL,           -- 0-indexed within session

    -- MESSAGE BOUNDARIES (references to messages table)
    start_sequence INTEGER NOT NULL,        -- First message sequence in turn
    end_sequence INTEGER NOT NULL,          -- Last message sequence in turn

    -- LLM-GENERATED CONTENT
    user_intent TEXT NOT NULL,              -- "User wanted to add JWT authentication"
    assistant_action TEXT NOT NULL,         -- "Created auth module with JWT support"
    summary TEXT NOT NULL,                  -- Combined summary sentence

    -- Classification
    turn_type TEXT,                         -- 'task', 'question', 'error_fix', 'clarification', 'discussion'

    -- Extracted entities (JSON arrays)
    key_topics TEXT,                        -- ["authentication", "JWT", "middleware"]
    decisions_made TEXT,                    -- ["Used RS256 over HS256", "Added refresh tokens"]
    code_concepts TEXT,                     -- ["error handling", "async/await", "middleware pattern"]

    -- CACHED TIMESTAMPS (derived from messages, cached for convenience)
    started_at TEXT NOT NULL,
    ended_at TEXT NOT NULL,

    -- GENERATION METADATA
    model_used TEXT,
    prompt_version INTEGER NOT NULL DEFAULT 1,
    generated_at TEXT NOT NULL DEFAULT (datetime('now', 'utc')),

    FOREIGN KEY (session_id) REFERENCES chat_sessions(id) ON DELETE CASCADE,
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

-- Indexes for turn_summaries
CREATE INDEX IF NOT EXISTS idx_turn_summaries_session ON turn_summaries(session_id);
CREATE INDEX IF NOT EXISTS idx_turn_summaries_type ON turn_summaries(turn_type);
CREATE INDEX IF NOT EXISTS idx_turn_summaries_started ON turn_summaries(started_at);

-- Indexes for session_summaries
CREATE INDEX IF NOT EXISTS idx_session_summaries_session ON session_summaries(session_id);
CREATE INDEX IF NOT EXISTS idx_session_summaries_outcome ON session_summaries(outcome);

-- Full-Text Search for turn_summaries
CREATE VIRTUAL TABLE IF NOT EXISTS turn_summaries_fts USING fts5(
    summary,
    user_intent,
    assistant_action,
    turn_id UNINDEXED,
    content='turn_summaries',
    content_rowid='rowid'
);

-- FTS triggers for turn_summaries
CREATE TRIGGER IF NOT EXISTS turn_summaries_fts_insert AFTER INSERT ON turn_summaries BEGIN
    INSERT INTO turn_summaries_fts(rowid, summary, user_intent, assistant_action, turn_id)
    VALUES (NEW.rowid, NEW.summary, NEW.user_intent, NEW.assistant_action, NEW.id);
END;

CREATE TRIGGER IF NOT EXISTS turn_summaries_fts_delete AFTER DELETE ON turn_summaries BEGIN
    INSERT INTO turn_summaries_fts(turn_summaries_fts, rowid, summary, user_intent, assistant_action, turn_id)
    VALUES('delete', OLD.rowid, OLD.summary, OLD.user_intent, OLD.assistant_action, OLD.id);
END;

CREATE TRIGGER IF NOT EXISTS turn_summaries_fts_update AFTER UPDATE ON turn_summaries BEGIN
    INSERT INTO turn_summaries_fts(turn_summaries_fts, rowid, summary, user_intent, assistant_action, turn_id)
    VALUES('delete', OLD.rowid, OLD.summary, OLD.user_intent, OLD.assistant_action, OLD.id);
    INSERT INTO turn_summaries_fts(rowid, summary, user_intent, assistant_action, turn_id)
    VALUES (NEW.rowid, NEW.summary, NEW.user_intent, NEW.assistant_action, NEW.id);
END;

-- Full-Text Search for session_summaries
CREATE VIRTUAL TABLE IF NOT EXISTS session_summaries_fts USING fts5(
    title,
    summary,
    primary_goal,
    session_id UNINDEXED,
    content='session_summaries',
    content_rowid='rowid'
);

-- FTS triggers for session_summaries
CREATE TRIGGER IF NOT EXISTS session_summaries_fts_insert AFTER INSERT ON session_summaries BEGIN
    INSERT INTO session_summaries_fts(rowid, title, summary, primary_goal, session_id)
    VALUES (NEW.rowid, NEW.title, NEW.summary, NEW.primary_goal, NEW.session_id);
END;

CREATE TRIGGER IF NOT EXISTS session_summaries_fts_delete AFTER DELETE ON session_summaries BEGIN
    INSERT INTO session_summaries_fts(session_summaries_fts, rowid, title, summary, primary_goal, session_id)
    VALUES('delete', OLD.rowid, OLD.title, OLD.summary, OLD.primary_goal, OLD.session_id);
END;

CREATE TRIGGER IF NOT EXISTS session_summaries_fts_update AFTER UPDATE ON session_summaries BEGIN
    INSERT INTO session_summaries_fts(session_summaries_fts, rowid, title, summary, primary_goal, session_id)
    VALUES('delete', OLD.rowid, OLD.title, OLD.summary, OLD.primary_goal, OLD.session_id);
    INSERT INTO session_summaries_fts(rowid, title, summary, primary_goal, session_id)
    VALUES (NEW.rowid, NEW.title, NEW.summary, NEW.primary_goal, NEW.session_id);
END;
