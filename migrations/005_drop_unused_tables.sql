-- Drop unused tables and schemas
-- Migration: 005_drop_unused_tables
-- Description: Remove unused tables to simplify database schema

-- Drop unused message tagging tables (from migration 002)
-- These tables were defined but never implemented in the codebase
DROP INDEX IF EXISTS idx_message_tag_relationships_tag_id;
DROP INDEX IF EXISTS idx_message_tag_relationships_message_id;
DROP INDEX IF EXISTS idx_message_tags_name;
DROP TRIGGER IF EXISTS update_message_tags_updated_at;
DROP TABLE IF EXISTS message_tag_relationships;
DROP TABLE IF EXISTS message_tags;

-- Drop unused analytics table (from migration 001)
-- This was superseded by the retrospection system
DROP INDEX IF EXISTS idx_usage_analyses_period;
DROP INDEX IF EXISTS idx_usage_analyses_type;
DROP TABLE IF EXISTS usage_analyses;
