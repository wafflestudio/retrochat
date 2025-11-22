-- Remove unused columns from analytics table: scores_json, metrics_json, processed_output_json
-- Migration: 015_remove_unused_analytics_columns
-- Description: Remove columns that are no longer used in the Rust model

-- SQLite doesn't support DROP COLUMN directly, so we need to recreate the table
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

    -- Metadata
    model_used TEXT,
    analysis_duration_ms INTEGER,

    FOREIGN KEY (analytics_request_id) REFERENCES analytics_requests(id) ON DELETE CASCADE
);

-- Step 2: Migrate existing data (excluding removed columns)
INSERT INTO analytics_new (
    id, analytics_request_id, session_id, generated_at,
    qualitative_output_json,
    ai_quantitative_output_json,
    model_used, analysis_duration_ms
)
SELECT
    id, analytics_request_id, session_id, generated_at,
    qualitative_output_json,
    ai_quantitative_output_json,
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
