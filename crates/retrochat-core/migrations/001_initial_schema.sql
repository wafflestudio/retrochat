-- Initial schema creation
-- Migration: 001_initial_schema
-- Description: Create initial database schema with all core tables

-- Schema version tracking table
CREATE TABLE IF NOT EXISTS schema_versions (
    version INTEGER PRIMARY KEY,
    applied_at TEXT NOT NULL DEFAULT (datetime('now', 'utc'))
);

-- Projects table
CREATE TABLE IF NOT EXISTS projects (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    description TEXT,
    working_directory TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now', 'utc')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now', 'utc')),
    session_count INTEGER NOT NULL DEFAULT 0,
    total_tokens INTEGER NOT NULL DEFAULT 0
);

-- Chat sessions table
CREATE TABLE IF NOT EXISTS chat_sessions (
    id TEXT PRIMARY KEY,
    provider TEXT NOT NULL,
    project_name TEXT,
    start_time TEXT NOT NULL,
    end_time TEXT,
    message_count INTEGER NOT NULL DEFAULT 0,
    token_count INTEGER,
    file_path TEXT NOT NULL,
    file_hash TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now', 'utc')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now', 'utc')),
    state TEXT NOT NULL DEFAULT 'Created',
    FOREIGN KEY (project_name) REFERENCES projects(name),
    UNIQUE(file_hash, file_path)
);

-- Messages table
CREATE TABLE IF NOT EXISTS messages (
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

-- Usage analysis table
CREATE TABLE IF NOT EXISTS usage_analyses (
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
    generated_at TEXT NOT NULL DEFAULT (datetime('now', 'utc'))
);

-- Provider configuration table
CREATE TABLE IF NOT EXISTS llm_providers (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    file_patterns TEXT NOT NULL, -- JSON array
    default_locations TEXT NOT NULL, -- JSON object
    parser_type TEXT NOT NULL,
    supports_tokens BOOLEAN NOT NULL DEFAULT FALSE,
    supports_tools BOOLEAN NOT NULL DEFAULT FALSE,
    last_updated TEXT NOT NULL DEFAULT (datetime('now', 'utc'))
);

-- Create indexes for performance
CREATE INDEX IF NOT EXISTS idx_chat_sessions_provider ON chat_sessions(provider);
CREATE INDEX IF NOT EXISTS idx_chat_sessions_project ON chat_sessions(project_name);
CREATE INDEX IF NOT EXISTS idx_chat_sessions_start_time ON chat_sessions(start_time);
CREATE INDEX IF NOT EXISTS idx_chat_sessions_file_hash ON chat_sessions(file_hash);

CREATE INDEX IF NOT EXISTS idx_messages_session_id ON messages(session_id);
CREATE INDEX IF NOT EXISTS idx_messages_role ON messages(role);
CREATE INDEX IF NOT EXISTS idx_messages_timestamp ON messages(timestamp);
CREATE INDEX IF NOT EXISTS idx_messages_sequence ON messages(session_id, sequence_number);

CREATE INDEX IF NOT EXISTS idx_projects_name ON projects(name);
CREATE INDEX IF NOT EXISTS idx_projects_created_at ON projects(created_at);

CREATE INDEX IF NOT EXISTS idx_usage_analyses_type ON usage_analyses(analysis_type);
CREATE INDEX IF NOT EXISTS idx_usage_analyses_period ON usage_analyses(time_period_start, time_period_end);

CREATE INDEX IF NOT EXISTS idx_llm_providers_name ON llm_providers(name);
CREATE INDEX IF NOT EXISTS idx_llm_providers_parser_type ON llm_providers(parser_type);

-- Create triggers for automatic updates
CREATE TRIGGER IF NOT EXISTS update_chat_sessions_updated_at
    AFTER UPDATE ON chat_sessions
    FOR EACH ROW
    BEGIN
        UPDATE chat_sessions SET updated_at = datetime('now', 'utc') WHERE id = NEW.id;
    END;

CREATE TRIGGER IF NOT EXISTS update_projects_updated_at
    AFTER UPDATE ON projects
    FOR EACH ROW
    BEGIN
        UPDATE projects SET updated_at = datetime('now', 'utc') WHERE id = NEW.id;
    END;

-- Create Full Text Search table for messages
CREATE VIRTUAL TABLE IF NOT EXISTS messages_fts USING fts5(
    content,
    session_id UNINDEXED,
    role UNINDEXED,
    timestamp UNINDEXED,
    content='messages',
    content_rowid='rowid'
);

-- Create triggers to maintain FTS table
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