-- Refactor tool storage schema
-- Migration: 009_refactor_tool_storage
-- Description: Restructure messages and tool_operations relationship:
--              - Add message_type to messages for clear classification
--              - Add tool_operation_id FK in messages (single direction)
--              - Store all tool data in tool_operations (remove from messages)
--              - Remove message_id/session_id from tool_operations (use reverse lookup)

-- =============================================================================
-- PHASE 1: No additional raw data columns needed
-- =============================================================================

-- We already have raw_input and raw_result which are sufficient:
-- - raw_input: stores ToolUse.input (tool parameters)
-- - raw_result: stores ToolResult.details
-- No need for redundant tool_use_raw, tool_result_raw, tool_result_content

-- =============================================================================
-- PHASE 2: No data migration needed
-- =============================================================================

-- raw_input and raw_result are already being populated during import
-- No additional data migration required

-- =============================================================================
-- PHASE 3: Drop FTS table and triggers FIRST (before modifying messages)
-- =============================================================================

-- We need to drop FTS triggers before modifying messages table
-- Otherwise we get "unsafe use of virtual table" errors

DROP TRIGGER IF EXISTS messages_fts_insert;
DROP TRIGGER IF EXISTS messages_fts_delete;
DROP TRIGGER IF EXISTS messages_fts_update;
DROP TABLE IF EXISTS messages_fts;

-- =============================================================================
-- PHASE 4: Add new columns to messages table
-- =============================================================================

-- Add message_type column (defaults to 'simple_message')
ALTER TABLE messages ADD COLUMN message_type TEXT NOT NULL DEFAULT 'simple_message';

-- Add tool_operation_id column (nullable FK to tool_operations)
ALTER TABLE messages ADD COLUMN tool_operation_id TEXT;

-- =============================================================================
-- PHASE 5: Set message_type based on existing data
-- =============================================================================

-- Set message_type to 'tool_request' for messages with tool_uses
UPDATE messages
SET message_type = 'tool_request'
WHERE tool_uses IS NOT NULL
  AND json_array_length(tool_uses) > 0;

-- Set message_type to 'tool_result' for messages with tool_results (and no tool_uses)
UPDATE messages
SET message_type = 'tool_result'
WHERE tool_results IS NOT NULL
  AND json_array_length(tool_results) > 0
  AND (tool_uses IS NULL OR json_array_length(tool_uses) = 0);

-- =============================================================================
-- PHASE 6: Link messages to tool_operations
-- =============================================================================

-- For each message, find its corresponding tool_operation and set the FK
-- Assuming 1 message = 1 tool_operation relationship as confirmed by user
UPDATE messages
SET tool_operation_id = (
    SELECT id
    FROM tool_operations
    WHERE tool_operations.message_id = messages.id
    LIMIT 1
)
WHERE EXISTS (
    SELECT 1
    FROM tool_operations
    WHERE tool_operations.message_id = messages.id
);

-- =============================================================================
-- PHASE 7: Create new indexes before dropping old ones
-- =============================================================================

-- Index for message_type queries
CREATE INDEX IF NOT EXISTS idx_messages_type ON messages(message_type);

-- Index for tool_operation_id FK
CREATE INDEX IF NOT EXISTS idx_messages_tool_operation ON messages(tool_operation_id);

-- Index for reverse lookup (finding messages by tool_operation)
CREATE INDEX IF NOT EXISTS idx_messages_tool_op_lookup ON messages(tool_operation_id)
    WHERE tool_operation_id IS NOT NULL;

-- =============================================================================
-- PHASE 8: Drop old columns from messages
-- =============================================================================

-- SQLite doesn't support DROP COLUMN directly, so we need to recreate the table
-- Create new messages table without tool_calls, tool_uses, tool_results

CREATE TABLE messages_new (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    role TEXT NOT NULL CHECK (role IN ('User', 'Assistant', 'System')),
    content TEXT NOT NULL CHECK (length(content) > 0),
    timestamp TEXT NOT NULL,
    token_count INTEGER CHECK (token_count >= 0),
    metadata TEXT,   -- JSON object
    sequence_number INTEGER NOT NULL,
    message_type TEXT NOT NULL DEFAULT 'simple_message' CHECK (message_type IN ('tool_request', 'tool_result', 'simple_message')),
    tool_operation_id TEXT,
    FOREIGN KEY (session_id) REFERENCES chat_sessions(id) ON DELETE CASCADE,
    FOREIGN KEY (tool_operation_id) REFERENCES tool_operations(id) ON DELETE SET NULL,
    UNIQUE(session_id, sequence_number)
);

-- Copy data from old table to new table
INSERT INTO messages_new (
    id, session_id, role, content, timestamp, token_count,
    metadata, sequence_number, message_type, tool_operation_id
)
SELECT
    id, session_id, role, content, timestamp, token_count,
    metadata, sequence_number, message_type, tool_operation_id
FROM messages;

-- Drop old table and rename new table
DROP TABLE messages;
ALTER TABLE messages_new RENAME TO messages;

-- =============================================================================
-- PHASE 9: Recreate indexes for messages table
-- =============================================================================

CREATE INDEX IF NOT EXISTS idx_messages_session_id ON messages(session_id);
CREATE INDEX IF NOT EXISTS idx_messages_role ON messages(role);
CREATE INDEX IF NOT EXISTS idx_messages_timestamp ON messages(timestamp);
CREATE INDEX IF NOT EXISTS idx_messages_sequence ON messages(session_id, sequence_number);
CREATE INDEX IF NOT EXISTS idx_messages_type ON messages(message_type);
CREATE INDEX IF NOT EXISTS idx_messages_tool_operation ON messages(tool_operation_id);

-- =============================================================================
-- PHASE 10: Recreate FTS table for messages (without tool columns)
-- =============================================================================

-- FTS table and triggers were already dropped in PHASE 3
-- Now recreate them with the new schema

-- Create FTS table
CREATE VIRTUAL TABLE IF NOT EXISTS messages_fts USING fts5(
    content,
    session_id UNINDEXED,
    role UNINDEXED,
    timestamp UNINDEXED,
    content='messages',
    content_rowid='rowid'
);

-- Recreate FTS triggers
CREATE TRIGGER IF NOT EXISTS messages_fts_insert AFTER INSERT ON messages BEGIN
    INSERT INTO messages_fts(rowid, content) VALUES (NEW.rowid, NEW.content);
END;

CREATE TRIGGER IF NOT EXISTS messages_fts_delete AFTER DELETE ON messages BEGIN
    INSERT INTO messages_fts(messages_fts, rowid, content) VALUES('delete', OLD.rowid, OLD.content);
END;

CREATE TRIGGER IF NOT EXISTS messages_fts_update AFTER UPDATE ON messages BEGIN
    INSERT INTO messages_fts(messages_fts, rowid, content) VALUES('delete', OLD.rowid, OLD.content);
    INSERT INTO messages_fts(rowid, content) VALUES (NEW.rowid, NEW.content);
END;

-- Rebuild FTS index with current data
INSERT INTO messages_fts(rowid, content) SELECT rowid, content FROM messages;

-- =============================================================================
-- PHASE 11: Drop message_id and session_id from tool_operations
-- =============================================================================

-- Create new tool_operations table without message_id and session_id
-- Consolidate all file-related columns into a single JSON column
CREATE TABLE tool_operations_new (
    id TEXT PRIMARY KEY,
    tool_use_id TEXT NOT NULL,
    tool_name TEXT NOT NULL,
    timestamp TEXT NOT NULL,

    -- File-related metadata as JSON (NULL for non-file tools)
    -- Schema: {file_path, file_extension?, is_code_file?, is_config_file?,
    --          lines_before?, lines_after?, lines_added?, lines_removed?,
    --          content_size?, is_bulk_edit?, is_refactoring?}
    file_metadata TEXT,  -- JSON object

    -- Generic fields for all tools
    success BOOLEAN,
    result_summary TEXT,
    raw_input TEXT,
    raw_result TEXT,

    created_at TEXT NOT NULL DEFAULT (datetime('now', 'utc'))
);

-- Copy data from old table to new table, converting file columns to JSON
INSERT INTO tool_operations_new (
    id, tool_use_id, tool_name, timestamp,
    file_metadata,
    success, result_summary, raw_input, raw_result,
    created_at
)
SELECT
    id, tool_use_id, tool_name, timestamp,
    -- Build file_metadata JSON only if file_path exists
    CASE
        WHEN file_path IS NOT NULL THEN
            json_object(
                'file_path', file_path,
                'file_extension', file_extension,
                'is_code_file', is_code_file,
                'is_config_file', is_config_file,
                'lines_before', lines_before,
                'lines_after', lines_after,
                'lines_added', lines_added,
                'lines_removed', lines_removed,
                'content_size', content_size,
                'is_bulk_edit', is_bulk_edit,
                'is_refactoring', is_refactoring
            )
        ELSE NULL
    END,
    success, result_summary, raw_input, raw_result,
    created_at
FROM tool_operations;

-- Drop old table and rename new table
DROP TABLE tool_operations;
ALTER TABLE tool_operations_new RENAME TO tool_operations;

-- =============================================================================
-- PHASE 12: Recreate indexes for tool_operations (excluding message/session)
-- =============================================================================

CREATE INDEX IF NOT EXISTS idx_tool_operations_tool_name ON tool_operations(tool_name);
CREATE INDEX IF NOT EXISTS idx_tool_operations_timestamp ON tool_operations(timestamp);

-- Index for file operations using JSON extraction
CREATE INDEX IF NOT EXISTS idx_tool_operations_has_file ON tool_operations(tool_name)
    WHERE file_metadata IS NOT NULL;

-- Note: Indexes for message_id and session_id are removed as those columns no longer exist
-- To query tool_operations by session: JOIN messages ON messages.tool_operation_id = tool_operations.id
-- Note: File-specific indexes (file_path, extension, code_files) are removed
-- SQLite JSON indexes would require generated columns which can be added if needed
