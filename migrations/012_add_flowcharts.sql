-- Add flowcharts table for storing session context flow analysis
CREATE TABLE flowcharts (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    nodes TEXT NOT NULL, -- JSON array of FlowchartNode
    edges TEXT NOT NULL, -- JSON array of FlowchartEdge
    created_at TEXT NOT NULL DEFAULT (datetime('now', 'utc')),
    token_usage INTEGER,
    FOREIGN KEY (session_id) REFERENCES chat_sessions(id) ON DELETE CASCADE
);

-- Indexes for performance
CREATE INDEX idx_flowcharts_session_id ON flowcharts(session_id);
CREATE INDEX idx_flowcharts_created_at ON flowcharts(created_at);
