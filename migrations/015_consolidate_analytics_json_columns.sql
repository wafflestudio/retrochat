-- Consolidate analytics columns into JSON groups
-- Migration: 015_consolidate_analytics_json_columns
-- Description: Group scores and metrics into JSON columns for better organization

-- Create new table with consolidated JSON columns
CREATE TABLE analytics_new (
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

-- Copy data from old table, grouping scores and metrics into JSON
INSERT INTO analytics_new (
    id, analytics_request_id, generated_at,
    scores_json, metrics_json,
    quantitative_input_json, qualitative_input_json,
    qualitative_output_json, processed_output_json,
    model_used, analysis_duration_ms
)
SELECT
    id,
    analytics_request_id,
    generated_at,
    -- Group scores into JSON
    json_object(
        'overall', overall_score,
        'code_quality', code_quality_score,
        'productivity', productivity_score,
        'efficiency', efficiency_score,
        'collaboration', collaboration_score,
        'learning', learning_score
    ) as scores_json,
    -- Group metrics into JSON
    json_object(
        'total_files_modified', total_files_modified,
        'total_files_read', total_files_read,
        'lines_added', lines_added,
        'lines_removed', lines_removed,
        'total_tokens_used', total_tokens_used,
        'session_duration_minutes', session_duration_minutes
    ) as metrics_json,
    quantitative_input_json,
    qualitative_input_json,
    qualitative_output_json,
    processed_output_json,
    model_used,
    analysis_duration_ms
FROM analytics;

-- Populate session_id from analytics_requests if not already set
-- Note: session_id might be NULL in old data, so we'll update it
UPDATE analytics_new
SET session_id = (
    SELECT session_id FROM analytics_requests
    WHERE analytics_requests.id = analytics_new.analytics_request_id
)
WHERE session_id IS NULL;

-- Drop old table
DROP TABLE analytics;

-- Rename new table
ALTER TABLE analytics_new RENAME TO analytics;

-- Recreate indexes
DROP INDEX IF EXISTS idx_analytics_request_id;
DROP INDEX IF EXISTS idx_analytics_generated_at;
DROP INDEX IF EXISTS idx_analytics_overall_score;
DROP INDEX IF EXISTS idx_analytics_productivity_score;

CREATE INDEX idx_analytics_request_id ON analytics(analytics_request_id);
CREATE INDEX idx_analytics_generated_at ON analytics(generated_at);
CREATE INDEX idx_analytics_session_id ON analytics(session_id);

-- Note: For querying scores/metrics, you can use JSON functions like:
-- SELECT * FROM analytics WHERE json_extract(scores_json, '$.overall') > 80
-- For better performance on JSON queries, consider creating generated columns:
-- ALTER TABLE analytics ADD COLUMN overall_score REAL GENERATED ALWAYS AS (json_extract(scores_json, '$.overall')) VIRTUAL;

