-- Extend existing analysis tables for advanced analytics
-- Migration: 011_extend_analysis_tables
-- Description: Create usage_analyses table and extend it for comprehensive session analysis

-- Create usage_analyses table if it doesn't exist
CREATE TABLE IF NOT EXISTS usage_analyses (
    id TEXT PRIMARY KEY,
    analysis_type TEXT NOT NULL,
    time_period_start TEXT NOT NULL,
    time_period_end TEXT NOT NULL,
    provider_filter TEXT,
    project_filter TEXT,
    total_sessions INTEGER NOT NULL DEFAULT 0,
    total_messages INTEGER NOT NULL DEFAULT 0,
    total_tokens INTEGER NOT NULL DEFAULT 0,
    average_session_length REAL NOT NULL DEFAULT 0,
    most_active_day TEXT,
    purpose_categories TEXT, -- JSON object
    quality_scores TEXT,     -- JSON object
    recommendations TEXT,    -- JSON array
    generated_at TEXT NOT NULL DEFAULT (datetime('now', 'utc'))
);

-- Add new columns to usage_analyses table for comprehensive analysis
ALTER TABLE usage_analyses ADD COLUMN session_id TEXT;
ALTER TABLE usage_analyses ADD COLUMN analysis_category TEXT DEFAULT 'comprehensive' CHECK (analysis_category IN ('comprehensive', 'quantitative', 'qualitative', 'processed', 'trends'));

-- Add quantitative metrics columns
ALTER TABLE usage_analyses ADD COLUMN file_operations_count INTEGER DEFAULT 0;
ALTER TABLE usage_analyses ADD COLUMN files_modified INTEGER DEFAULT 0;
ALTER TABLE usage_analyses ADD COLUMN files_created INTEGER DEFAULT 0;
ALTER TABLE usage_analyses ADD COLUMN files_read INTEGER DEFAULT 0;
ALTER TABLE usage_analyses ADD COLUMN lines_added INTEGER DEFAULT 0;
ALTER TABLE usage_analyses ADD COLUMN lines_removed INTEGER DEFAULT 0;
ALTER TABLE usage_analyses ADD COLUMN refactoring_operations INTEGER DEFAULT 0;
ALTER TABLE usage_analyses ADD COLUMN bulk_edit_operations INTEGER DEFAULT 0;

-- Add qualitative analysis columns (JSON storage)
ALTER TABLE usage_analyses ADD COLUMN quantitative_scores TEXT; -- JSON: {overall: 85, code_quality: 78, productivity: 92, ...}
ALTER TABLE usage_analyses ADD COLUMN qualitative_insights TEXT; -- JSON: {insights: [...], good_patterns: [...], improvements: [...]}
ALTER TABLE usage_analyses ADD COLUMN processed_metrics TEXT; -- JSON: {session_metrics: {...}, token_metrics: {...}, code_metrics: {...}}

-- Add analysis metadata
ALTER TABLE usage_analyses ADD COLUMN analysis_prompt TEXT; -- Store the prompt used for analysis
ALTER TABLE usage_analyses ADD COLUMN ai_model_used TEXT; -- Store which AI model was used
ALTER TABLE usage_analyses ADD COLUMN analysis_duration_ms INTEGER; -- How long the analysis took

-- Create indexes for new columns
CREATE INDEX IF NOT EXISTS idx_usage_analyses_session_id ON usage_analyses(session_id);
CREATE INDEX IF NOT EXISTS idx_usage_analyses_category ON usage_analyses(analysis_category);
CREATE INDEX IF NOT EXISTS idx_usage_analyses_generated_at ON usage_analyses(generated_at);

-- Add foreign key constraint for session_id
-- Note: We'll add this after ensuring all existing data is compatible
-- ALTER TABLE usage_analyses ADD CONSTRAINT fk_usage_analyses_session_id 
--     FOREIGN KEY (session_id) REFERENCES chat_sessions(id) ON DELETE CASCADE;