-- Add retrospect_requests table for tracking retrospection analysis requests
CREATE TABLE retrospect_requests (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    analysis_type TEXT NOT NULL CHECK (analysis_type IN ('user-interaction', 'collaboration', 'question-quality', 'task-breakdown', 'follow-up', 'custom')),
    status TEXT NOT NULL CHECK (status IN ('pending', 'running', 'completed', 'failed', 'cancelled')),
    started_at TEXT NOT NULL DEFAULT (datetime('now', 'utc')),
    completed_at TEXT,
    created_by TEXT,
    error_message TEXT,
    custom_prompt TEXT,
    FOREIGN KEY (session_id) REFERENCES chat_sessions(id) ON DELETE CASCADE
);

-- Indexes for performance
CREATE INDEX idx_retrospect_requests_status ON retrospect_requests(status);
CREATE INDEX idx_retrospect_requests_session_id ON retrospect_requests(session_id);
CREATE INDEX idx_retrospect_requests_created_by ON retrospect_requests(created_by);
CREATE INDEX idx_retrospect_requests_started_at ON retrospect_requests(started_at);