-- Remove analysis_type column from retrospect_requests table
-- SQLite doesn't support DROP COLUMN directly, so we need to recreate the table

-- Create new table without analysis_type
CREATE TABLE retrospect_requests_new (
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

-- Copy data from old table to new table (excluding analysis_type)
INSERT INTO retrospect_requests_new (id, session_id, status, started_at, completed_at, created_by, error_message, custom_prompt)
SELECT id, session_id, status, started_at, completed_at, created_by, error_message, custom_prompt
FROM retrospect_requests;

-- Drop old table
DROP TABLE retrospect_requests;

-- Rename new table to original name
ALTER TABLE retrospect_requests_new RENAME TO retrospect_requests;

-- Recreate indexes
CREATE INDEX idx_retrospect_requests_status ON retrospect_requests(status);
CREATE INDEX idx_retrospect_requests_session_id ON retrospect_requests(session_id);
CREATE INDEX idx_retrospect_requests_created_by ON retrospect_requests(created_by);
CREATE INDEX idx_retrospect_requests_started_at ON retrospect_requests(started_at);
