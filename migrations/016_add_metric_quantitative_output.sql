-- Add metric_quantitative_output_json column to analytics table
-- Migration: 016_add_metric_quantitative_output
-- Description: Add column to store metric-based quantitative output (file changes, time metrics, token metrics, tool usage)

-- SQLite doesn't support ADD COLUMN with complex defaults properly, so we recreate the table
-- Step 1: Create new table with updated schema
CREATE TABLE analytics_new (
    id TEXT PRIMARY KEY,
    analytics_request_id TEXT NOT NULL,
    session_id TEXT,
    generated_at TEXT NOT NULL,

    -- Analysis output data
    qualitative_output_json TEXT NOT NULL,

    -- AI-generated quantitative output (rubric-based LLM-as-a-judge evaluation)
    ai_quantitative_output_json TEXT NOT NULL DEFAULT '{"rubric_scores":[],"rubric_summary":null}',

    -- Metric-based quantitative output (file changes, time metrics, token metrics, tool usage)
    metric_quantitative_output_json TEXT NOT NULL DEFAULT '{"file_changes":{"total_files_modified":0,"total_files_read":0,"lines_added":0,"lines_removed":0,"net_code_growth":0},"time_metrics":{"total_session_time_minutes":0.0,"peak_hours":[]},"token_metrics":{"total_tokens_used":0,"input_tokens":0,"output_tokens":0,"token_efficiency":0.0},"tool_usage":{"total_operations":0,"successful_operations":0,"failed_operations":0,"tool_distribution":{},"average_execution_time_ms":0.0}}',

    -- Metadata
    model_used TEXT,
    analysis_duration_ms INTEGER,

    FOREIGN KEY (analytics_request_id) REFERENCES analytics_requests(id) ON DELETE CASCADE
);

-- Step 2: Migrate existing data
INSERT INTO analytics_new (
    id, analytics_request_id, session_id, generated_at,
    qualitative_output_json,
    ai_quantitative_output_json,
    metric_quantitative_output_json,
    model_used, analysis_duration_ms
)
SELECT
    id, analytics_request_id, session_id, generated_at,
    qualitative_output_json,
    ai_quantitative_output_json,
    '{"file_changes":{"total_files_modified":0,"total_files_read":0,"lines_added":0,"lines_removed":0,"net_code_growth":0},"time_metrics":{"total_session_time_minutes":0.0,"peak_hours":[]},"token_metrics":{"total_tokens_used":0,"input_tokens":0,"output_tokens":0,"token_efficiency":0.0},"tool_usage":{"total_operations":0,"successful_operations":0,"failed_operations":0,"tool_distribution":{},"average_execution_time_ms":0.0}}' as metric_quantitative_output_json,
    model_used, analysis_duration_ms
FROM analytics;

-- Step 3: Drop old table
DROP TABLE analytics;

-- Step 4: Rename new table
ALTER TABLE analytics_new RENAME TO analytics;

-- Step 5: Recreate indexes
CREATE INDEX idx_analytics_request_id ON analytics(analytics_request_id);
CREATE INDEX idx_analytics_generated_at ON analytics(generated_at);
CREATE INDEX idx_analytics_session_id ON analytics(session_id);
