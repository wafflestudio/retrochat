-- Add thinking message type
-- Migration: 013_add_thinking_message_type
-- Description: Add 'thinking' to message_type CHECK constraint to support Claude Code thinking messages

-- Drop the old constraint and add the new one with 'thinking'
-- SQLite doesn't support ALTER TABLE ... DROP CONSTRAINT, so we need to recreate the table

-- Create a temporary table with the new schema
CREATE TABLE messages_new (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    role TEXT NOT NULL CHECK (role IN ('User', 'Assistant', 'System')),
    content TEXT NOT NULL CHECK (length(content) > 0),
    timestamp TEXT NOT NULL,
    token_count INTEGER CHECK (token_count >= 0),
    metadata TEXT,   -- JSON object
    sequence_number INTEGER NOT NULL,
    message_type TEXT NOT NULL DEFAULT 'simple_message' CHECK (message_type IN ('tool_request', 'tool_result', 'thinking', 'simple_message')),
    tool_operation_id TEXT,
    FOREIGN KEY (session_id) REFERENCES chat_sessions(id) ON DELETE CASCADE,
    FOREIGN KEY (tool_operation_id) REFERENCES tool_operations(id) ON DELETE SET NULL,
    UNIQUE(session_id, sequence_number)
);

-- Copy data from old table
INSERT INTO messages_new (id, session_id, role, content, timestamp, token_count, metadata, sequence_number, message_type, tool_operation_id)
SELECT id, session_id, role, content, timestamp, token_count, metadata, sequence_number, message_type, tool_operation_id
FROM messages;

-- Drop old table
DROP TABLE messages;

-- Rename new table to messages
ALTER TABLE messages_new RENAME TO messages;

-- Recreate indexes
CREATE INDEX IF NOT EXISTS idx_messages_session_id ON messages(session_id);
CREATE INDEX IF NOT EXISTS idx_messages_role ON messages(role);
CREATE INDEX IF NOT EXISTS idx_messages_timestamp ON messages(timestamp);
CREATE INDEX IF NOT EXISTS idx_messages_sequence ON messages(session_id, sequence_number);
CREATE INDEX IF NOT EXISTS idx_messages_message_type ON messages(message_type);
CREATE INDEX IF NOT EXISTS idx_messages_tool_operation ON messages(tool_operation_id);

-- Rebuild FTS index after table recreation
-- First, delete and recreate the FTS table
DROP TABLE IF EXISTS messages_fts;

CREATE VIRTUAL TABLE messages_fts USING fts5(
    content,
    session_id UNINDEXED,
    role UNINDEXED,
    timestamp UNINDEXED,
    content='messages',
    content_rowid='rowid'
);

-- Recreate FTS triggers
CREATE TRIGGER messages_fts_insert AFTER INSERT ON messages BEGIN
    INSERT INTO messages_fts(rowid, content) VALUES (NEW.rowid, NEW.content);
END;

CREATE TRIGGER messages_fts_delete AFTER DELETE ON messages BEGIN
    INSERT INTO messages_fts(messages_fts, rowid, content) VALUES('delete', OLD.rowid, OLD.content);
END;

CREATE TRIGGER messages_fts_update AFTER UPDATE ON messages BEGIN
    INSERT INTO messages_fts(messages_fts, rowid, content) VALUES('delete', OLD.rowid, OLD.content);
    INSERT INTO messages_fts(rowid, content) VALUES (NEW.rowid, NEW.content);
END;
