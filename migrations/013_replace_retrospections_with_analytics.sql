-- Replace retrospections with analytics_retrospections
-- Migration: 013_replace_retrospections_with_analytics
-- Description: Drop old retrospections and rename analytics_retrospections, add retrospect_request_id

-- Drop old retrospections table
DROP TABLE IF EXISTS retrospections;

-- Rename analytics_retrospections to retrospections
ALTER TABLE analytics_retrospections RENAME TO retrospections;

-- Add retrospect_request_id column and populate it
-- We'll create a retrospect_request for each existing retrospection
ALTER TABLE retrospections ADD COLUMN retrospect_request_id TEXT;

-- Create retrospect_requests for existing retrospections (if any)
INSERT INTO retrospect_requests (id, session_id, status, started_at, completed_at, created_by)
SELECT
    r.id || '-request',
    r.session_id,
    'completed',
    r.generated_at,
    r.generated_at,
    'system-migration'
FROM retrospections r
WHERE NOT EXISTS (
    SELECT 1 FROM retrospect_requests rr WHERE rr.session_id = r.session_id
);

-- Update retrospections with request_id
UPDATE retrospections
SET retrospect_request_id = (
    SELECT id FROM retrospect_requests
    WHERE session_id = retrospections.session_id
    LIMIT 1
);

-- Make retrospect_request_id NOT NULL and add foreign key
-- First create a new table with the constraint
CREATE TABLE retrospections_new (
    id TEXT PRIMARY KEY,
    retrospect_request_id TEXT NOT NULL,
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

    FOREIGN KEY (retrospect_request_id) REFERENCES retrospect_requests(id) ON DELETE CASCADE
);

-- Copy data
INSERT INTO retrospections_new
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
FROM retrospections;

-- Drop old table
DROP TABLE retrospections;

-- Rename new table
ALTER TABLE retrospections_new RENAME TO retrospections;

-- Recreate indexes
CREATE INDEX idx_retrospections_request_id ON retrospections(retrospect_request_id);
CREATE INDEX idx_retrospections_generated_at ON retrospections(generated_at);
CREATE INDEX idx_retrospections_overall_score ON retrospections(overall_score);
CREATE INDEX idx_retrospections_productivity_score ON retrospections(productivity_score);
