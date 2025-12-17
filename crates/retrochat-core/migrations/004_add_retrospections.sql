-- Add retrospections table for storing analysis results
CREATE TABLE retrospections (
    id TEXT PRIMARY KEY,
    retrospect_request_id TEXT NOT NULL,
    response_text TEXT NOT NULL,
    token_usage INTEGER,
    response_time_ms INTEGER,
    model_used TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now', 'utc')),
    metadata TEXT, -- JSON
    FOREIGN KEY (retrospect_request_id) REFERENCES retrospect_requests(id) ON DELETE CASCADE
);

-- Indexes for performance
CREATE INDEX idx_retrospections_request_id ON retrospections(retrospect_request_id);
CREATE INDEX idx_retrospections_created_at ON retrospections(created_at);