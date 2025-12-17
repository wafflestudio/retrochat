-- Add message tags functionality
-- Migration: 002_add_message_tags
-- Description: Add tags table and message-tag relationships

-- Create tags table
CREATE TABLE IF NOT EXISTS message_tags (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    color TEXT NOT NULL DEFAULT '#007ACC',
    description TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now', 'utc')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now', 'utc'))
);

-- Create message-tag relationship table
CREATE TABLE IF NOT EXISTS message_tag_relationships (
    message_id TEXT NOT NULL,
    tag_id TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now', 'utc')),
    PRIMARY KEY (message_id, tag_id),
    FOREIGN KEY (message_id) REFERENCES messages(id) ON DELETE CASCADE,
    FOREIGN KEY (tag_id) REFERENCES message_tags(id) ON DELETE CASCADE
);

-- Create indexes for performance
CREATE INDEX IF NOT EXISTS idx_message_tags_name ON message_tags(name);
CREATE INDEX IF NOT EXISTS idx_message_tag_relationships_message_id ON message_tag_relationships(message_id);
CREATE INDEX IF NOT EXISTS idx_message_tag_relationships_tag_id ON message_tag_relationships(tag_id);

-- Create trigger for tag updated_at
CREATE TRIGGER IF NOT EXISTS update_message_tags_updated_at
    AFTER UPDATE ON message_tags
    FOR EACH ROW
    BEGIN
        UPDATE message_tags SET updated_at = datetime('now', 'utc') WHERE id = NEW.id;
    END;