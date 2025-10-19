-- Add project_path column to chat_sessions table
-- This stores the working directory path where the chat session was executed
ALTER TABLE chat_sessions ADD COLUMN project_path TEXT;

-- Create index for faster queries on project_path
CREATE INDEX IF NOT EXISTS idx_chat_sessions_project_path ON chat_sessions(project_path);
