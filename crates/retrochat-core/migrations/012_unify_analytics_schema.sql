-- Unify analytics schema: Create analytics_requests and analytics tables
-- Migration: 012_unify_analytics_schema
-- Description: Replace retrospections with unified analytics schema using analytics naming
--              This migration combines the functionality of:
--              - 012: Create analytics_retrospections table
--              - 013: Add request_id relationship
--              - 014: Rename to analytics naming
--              - 015: Consolidate JSON columns for scores and metrics

-- Drop unused tables
DROP TABLE IF EXISTS usage_analyses;

-- Step 1: Rename retrospect_requests to analytics_requests and remove analysis_type
-- Create analytics_requests table (without analysis_type column)
CREATE TABLE analytics_requests (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('pending', 'running', 'completed', 'failed', 'cancelled')),
    started_at TEXT NOT NULL DEFAULT (datetime('now', 'utc')),
    completed_at TEXT,
    created_by TEXT,
    error_message TEXT,
    custom_prompt TEXT,
    FOREIGN KEY (session_id) REFERENCES chat_sessions(id) ON DELETE CASCADE
);

-- Migrate data from retrospect_requests to analytics_requests (if exists)
INSERT INTO analytics_requests (id, session_id, status, started_at, completed_at, created_by, error_message, custom_prompt)
SELECT id, session_id, status, started_at, completed_at, created_by, error_message, custom_prompt
FROM retrospect_requests
WHERE NOT EXISTS (SELECT 1 FROM analytics_requests WHERE analytics_requests.id = retrospect_requests.id);

-- Create indexes for analytics_requests
CREATE INDEX idx_analytics_requests_status ON analytics_requests(status);
CREATE INDEX idx_analytics_requests_session_id ON analytics_requests(session_id);
CREATE INDEX idx_analytics_requests_created_by ON analytics_requests(created_by);
CREATE INDEX idx_analytics_requests_started_at ON analytics_requests(started_at);

-- Step 2: Create analytics table with final schema (JSON columns for scores and metrics)
CREATE TABLE analytics (
    id TEXT PRIMARY KEY,
    analytics_request_id TEXT NOT NULL,
    session_id TEXT,
    generated_at TEXT NOT NULL,

    -- Consolidated JSON columns (meaningful groups)
    scores_json TEXT NOT NULL,           -- { overall, code_quality, productivity, efficiency, collaboration, learning }
    metrics_json TEXT NOT NULL,          -- { total_files_modified, total_files_read, lines_added, lines_removed, total_tokens_used, session_duration_minutes }
    
    -- Analysis data (already JSON)
    quantitative_input_json TEXT NOT NULL,
    qualitative_input_json TEXT NOT NULL,
    qualitative_output_json TEXT NOT NULL,
    processed_output_json TEXT NOT NULL,

    -- Metadata
    model_used TEXT,
    analysis_duration_ms INTEGER,

    FOREIGN KEY (analytics_request_id) REFERENCES analytics_requests(id) ON DELETE CASCADE
);

-- Step 3: Migrate existing data from retrospections (if exists)
-- Note: Old retrospections table had different schema, so we'll try to migrate what we can
-- For old retrospections without full analysis data, we'll create empty JSON structures
INSERT INTO analytics (
    id,
    analytics_request_id,
    session_id,
    generated_at,
    scores_json,
    metrics_json,
    quantitative_input_json,
    qualitative_input_json,
    qualitative_output_json,
    processed_output_json,
    model_used,
    analysis_duration_ms
)
SELECT
    r.id,
    COALESCE(
        (SELECT id FROM analytics_requests WHERE session_id = (
            SELECT session_id FROM retrospect_requests WHERE id = r.retrospect_request_id
        ) LIMIT 1),
        r.retrospect_request_id || '-migrated'
    ) as analytics_request_id,
    COALESCE(
        (SELECT session_id FROM retrospect_requests WHERE id = r.retrospect_request_id),
        'unknown'
    ) as session_id,
    COALESCE(r.created_at, datetime('now', 'utc')) as generated_at,
    -- Default scores if not available
    json_object(
        'overall', 0.0,
        'code_quality', 0.0,
        'productivity', 0.0,
        'efficiency', 0.0,
        'collaboration', 0.0,
        'learning', 0.0
    ) as scores_json,
    -- Default metrics if not available
    json_object(
        'total_files_modified', 0,
        'total_files_read', 0,
        'lines_added', 0,
        'lines_removed', 0,
        'total_tokens_used', COALESCE(r.token_usage, 0),
        'session_duration_minutes', 0.0
    ) as metrics_json,
    -- Empty JSON structures for missing data
    json_object() as quantitative_input_json,
    json_object() as qualitative_input_json,
    COALESCE(r.metadata, json_object()) as qualitative_output_json,
    json_object() as processed_output_json,
    r.model_used,
    r.response_time_ms as analysis_duration_ms
FROM retrospections r
WHERE NOT EXISTS (SELECT 1 FROM analytics WHERE analytics.id = r.id);

-- Populate session_id from analytics_requests if not already set
UPDATE analytics
SET session_id = (
    SELECT session_id FROM analytics_requests
    WHERE analytics_requests.id = analytics.analytics_request_id
)
WHERE session_id IS NULL OR session_id = 'unknown';

-- Step 4: Drop old tables after migration
DROP TABLE IF EXISTS retrospections;
DROP TABLE IF EXISTS retrospect_requests;

-- Create indexes for analytics
CREATE INDEX idx_analytics_request_id ON analytics(analytics_request_id);
CREATE INDEX idx_analytics_generated_at ON analytics(generated_at);
CREATE INDEX idx_analytics_session_id ON analytics(session_id);

-- Note: For querying scores/metrics, you can use JSON functions like:
-- SELECT * FROM analytics WHERE json_extract(scores_json, '$.overall') > 80
-- For better performance on JSON queries, consider creating generated columns:
-- ALTER TABLE analytics ADD COLUMN overall_score REAL GENERATED ALWAYS AS (json_extract(scores_json, '$.overall')) VIRTUAL;

