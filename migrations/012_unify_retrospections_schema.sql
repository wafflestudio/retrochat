-- Unify retrospections schema to store ComprehensiveAnalysis
-- Migration: 012_unify_retrospections_schema
-- Description: Create analytics_retrospections table for ComprehensiveAnalysis

-- Drop unused tables
DROP TABLE IF EXISTS usage_analyses;

-- Create new analytics_retrospections table with comprehensive analysis support
CREATE TABLE analytics_retrospections (
    -- Primary identification
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,

    -- Timestamps
    generated_at TEXT NOT NULL,

    -- Quantitative scores (for queries/filtering)
    overall_score REAL NOT NULL,
    code_quality_score REAL NOT NULL,
    productivity_score REAL NOT NULL,
    efficiency_score REAL NOT NULL,
    collaboration_score REAL NOT NULL,
    learning_score REAL NOT NULL,

    -- Key metrics (for queries/aggregation)
    total_files_modified INTEGER NOT NULL DEFAULT 0,
    total_files_read INTEGER NOT NULL DEFAULT 0,
    lines_added INTEGER NOT NULL DEFAULT 0,
    lines_removed INTEGER NOT NULL DEFAULT 0,
    total_tokens_used INTEGER NOT NULL DEFAULT 0,
    session_duration_minutes REAL NOT NULL DEFAULT 0,

    -- Full analysis data (JSON storage)
    quantitative_input_json TEXT NOT NULL,
    qualitative_input_json TEXT NOT NULL,
    qualitative_output_json TEXT NOT NULL,
    processed_output_json TEXT NOT NULL,

    -- Metadata
    model_used TEXT,
    analysis_duration_ms INTEGER,

    FOREIGN KEY (session_id) REFERENCES chat_sessions(id) ON DELETE CASCADE
);

-- Indexes for performance
CREATE INDEX idx_analytics_retrospections_session_id ON analytics_retrospections(session_id);
CREATE INDEX idx_analytics_retrospections_generated_at ON analytics_retrospections(generated_at);
CREATE INDEX idx_analytics_retrospections_overall_score ON analytics_retrospections(overall_score);
CREATE INDEX idx_analytics_retrospections_productivity_score ON analytics_retrospections(productivity_score);
