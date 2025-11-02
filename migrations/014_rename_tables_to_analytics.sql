-- Rename tables to use analytics naming
-- Migration: 014_rename_tables_to_analytics
-- Description: Rename retrospect_requests to analytics_requests and retrospections to analytics

-- Rename retrospect_requests to analytics_requests
ALTER TABLE retrospect_requests RENAME TO analytics_requests;

-- Rename retrospections to analytics
ALTER TABLE retrospections RENAME TO analytics;

-- Rename the column retrospect_request_id to analytics_request_id
-- SQLite doesn't support ALTER COLUMN, so we need to recreate the table
CREATE TABLE analytics_new (
    id TEXT PRIMARY KEY,
    analytics_request_id TEXT NOT NULL,
    generated_at TEXT NOT NULL,

    overall_score REAL NOT NULL,
    code_quality_score REAL NOT NULL,
    productivity_score REAL NOT NULL,
    efficiency_score REAL NOT NULL,
    collaboration_score REAL NOT NULL,
    learning_score REAL NOT NULL,

    total_files_modified INTEGER NOT NULL DEFAULT 0,
    total_files_read INTEGER NOT NULL DEFAULT 0,
    lines_added INTEGER NOT NULL DEFAULT 0,
    lines_removed INTEGER NOT NULL DEFAULT 0,
    total_tokens_used INTEGER NOT NULL DEFAULT 0,
    session_duration_minutes REAL NOT NULL DEFAULT 0,

    quantitative_input_json TEXT NOT NULL,
    qualitative_input_json TEXT NOT NULL,
    qualitative_output_json TEXT NOT NULL,
    processed_output_json TEXT NOT NULL,

    model_used TEXT,
    analysis_duration_ms INTEGER,

    FOREIGN KEY (analytics_request_id) REFERENCES analytics_requests(id) ON DELETE CASCADE
);

-- Copy data with column rename
INSERT INTO analytics_new
SELECT
    id, retrospect_request_id, generated_at,
    overall_score, code_quality_score, productivity_score,
    efficiency_score, collaboration_score, learning_score,
    total_files_modified, total_files_read,
    lines_added, lines_removed,
    total_tokens_used, session_duration_minutes,
    quantitative_input_json, qualitative_input_json,
    qualitative_output_json, processed_output_json,
    model_used, analysis_duration_ms
FROM analytics;

-- Drop old table
DROP TABLE analytics;

-- Rename new table
ALTER TABLE analytics_new RENAME TO analytics;

-- Recreate indexes with new names
DROP INDEX IF EXISTS idx_retrospections_request_id;
DROP INDEX IF EXISTS idx_retrospections_generated_at;
DROP INDEX IF EXISTS idx_retrospections_overall_score;
DROP INDEX IF EXISTS idx_retrospections_productivity_score;

CREATE INDEX idx_analytics_request_id ON analytics(analytics_request_id);
CREATE INDEX idx_analytics_generated_at ON analytics(generated_at);
CREATE INDEX idx_analytics_overall_score ON analytics(overall_score);
CREATE INDEX idx_analytics_productivity_score ON analytics(productivity_score);

-- Rename indexes for analytics_requests
DROP INDEX IF EXISTS idx_retrospect_requests_status;
DROP INDEX IF EXISTS idx_retrospect_requests_session_id;
DROP INDEX IF EXISTS idx_retrospect_requests_created_by;
DROP INDEX IF EXISTS idx_retrospect_requests_started_at;

CREATE INDEX idx_analytics_requests_status ON analytics_requests(status);
CREATE INDEX idx_analytics_requests_session_id ON analytics_requests(session_id);
CREATE INDEX idx_analytics_requests_created_by ON analytics_requests(created_by);
CREATE INDEX idx_analytics_requests_started_at ON analytics_requests(started_at);

