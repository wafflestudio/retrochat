-- Add embedding column to messages table
-- Migration: 008_add_message_embeddings
-- Description: Add embedding vector column (768 dimensions) for semantic search

-- Add embedding column to messages table (can be NULL for now)
-- Using BLOB to store float32 vectors of 768 dimensions
ALTER TABLE messages ADD COLUMN embedding BLOB;

-- Create virtual table for vector similarity search using sqlite-vec
-- Note: This will be created dynamically when needed in the application code
-- since sqlite-vec needs to be loaded as an extension first
