-- Add unified tool storage columns
-- Migration: 006_add_unified_tool_storage
-- Description: Add tool_uses and tool_results columns for vendor-agnostic tool storage

-- Add new columns for unified tool storage
ALTER TABLE messages ADD COLUMN tool_uses TEXT;      -- JSON array of unified tool requests
ALTER TABLE messages ADD COLUMN tool_results TEXT;   -- JSON array of unified tool responses

-- Create indexes for tool usage queries
CREATE INDEX IF NOT EXISTS idx_messages_tool_uses ON messages(tool_uses) WHERE tool_uses IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_messages_tool_results ON messages(tool_results) WHERE tool_results IS NOT NULL;

-- Note: The existing 'tool_calls' column is kept for backward compatibility during migration
-- It will be populated by parsers for a transition period before eventual removal
