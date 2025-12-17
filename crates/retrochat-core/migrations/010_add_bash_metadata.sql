-- Add bash metadata JSON column
-- Migration: 010_add_bash_metadata
-- Description: Add single JSON column for bash-specific metadata

-- Add bash metadata JSON column to tool_operations table
ALTER TABLE tool_operations ADD COLUMN bash_metadata TEXT;

-- Create index for bash operations
CREATE INDEX IF NOT EXISTS idx_tool_operations_bash_metadata ON tool_operations(tool_name)
    WHERE tool_name = 'Bash' AND bash_metadata IS NOT NULL;