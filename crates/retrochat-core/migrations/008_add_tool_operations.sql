-- Add tool_operations table for tracking all tool usage
-- Migration: 008_add_tool_operations
-- Description: Create unified table to track all tool operations (Read, Write, Edit, etc.)
--              with specialized columns for file operations

-- Create tool_operations table
CREATE TABLE IF NOT EXISTS tool_operations (
    id TEXT PRIMARY KEY,
    message_id TEXT NOT NULL,
    tool_use_id TEXT NOT NULL,
    session_id TEXT NOT NULL,
    tool_name TEXT NOT NULL,  -- "Read", "Write", "Edit", "Bash", "Task", etc.
    timestamp TEXT NOT NULL,

    -- File-related fields (NULL for non-file tools)
    file_path TEXT,
    file_extension TEXT,
    is_code_file BOOLEAN,
    is_config_file BOOLEAN,

    -- Line change metrics (NULL for non-file tools or non-applicable operations)
    lines_before INTEGER,      -- Edit: number of lines in old_string
    lines_after INTEGER,       -- Edit/Write: number of lines in new_string/content
    lines_added INTEGER,       -- Edit: lines added (lines_after - lines_before if positive)
    lines_removed INTEGER,     -- Edit: lines removed (lines_before - lines_after if positive)
    content_size INTEGER,      -- Write: content size in bytes, Read: file size if available

    -- Edit-specific flags
    is_bulk_edit BOOLEAN,      -- Edit: replace_all flag
    is_refactoring BOOLEAN,    -- Edit: refactoring heuristic detection

    -- Generic fields for all tools
    success BOOLEAN,           -- Opposite of ToolResult.is_error
    result_summary TEXT,       -- Brief summary from ToolResult.content
    raw_input TEXT,            -- JSON: ToolUse.input (full original input)
    raw_result TEXT,           -- JSON: ToolResult.content/details (full original result)

    created_at TEXT NOT NULL DEFAULT (datetime('now', 'utc')),

    FOREIGN KEY (message_id) REFERENCES messages(id) ON DELETE CASCADE,
    FOREIGN KEY (session_id) REFERENCES chat_sessions(id) ON DELETE CASCADE
);

-- Create indexes for efficient querying
CREATE INDEX IF NOT EXISTS idx_tool_operations_message ON tool_operations(message_id);
CREATE INDEX IF NOT EXISTS idx_tool_operations_session ON tool_operations(session_id);
CREATE INDEX IF NOT EXISTS idx_tool_operations_tool_name ON tool_operations(tool_name);
CREATE INDEX IF NOT EXISTS idx_tool_operations_file_path ON tool_operations(file_path);
CREATE INDEX IF NOT EXISTS idx_tool_operations_extension ON tool_operations(file_extension);
CREATE INDEX IF NOT EXISTS idx_tool_operations_timestamp ON tool_operations(timestamp);

-- Composite index for quick file operations queries
CREATE INDEX IF NOT EXISTS idx_tool_operations_file_ops ON tool_operations(tool_name, file_path)
    WHERE file_path IS NOT NULL;

-- Index for code file operations
CREATE INDEX IF NOT EXISTS idx_tool_operations_code_files ON tool_operations(is_code_file, tool_name)
    WHERE is_code_file = TRUE;
