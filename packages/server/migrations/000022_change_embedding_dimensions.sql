-- Change embedding dimensions from 1536 (OpenAI) to 1024 (Voyage AI)
-- This migration updates vector dimensions for the switch to voyage-3-large

-- Drop existing indexes first
DROP INDEX IF EXISTS idx_members_embedding;
DROP INDEX IF EXISTS idx_needs_embedding;

-- Drop existing embedding columns
ALTER TABLE members
    DROP COLUMN IF EXISTS embedding;

ALTER TABLE organization_needs
    DROP COLUMN IF EXISTS embedding;

-- Add new embedding columns with 1024 dimensions (voyage-3-large)
ALTER TABLE members
    ADD COLUMN embedding vector(1024);

ALTER TABLE organization_needs
    ADD COLUMN embedding vector(1024);

-- Create indexes for fast cosine similarity search
CREATE INDEX idx_members_embedding ON members
    USING ivfflat (embedding vector_cosine_ops)
    WITH (lists = 100);

CREATE INDEX idx_needs_embedding ON organization_needs
    USING ivfflat (embedding vector_cosine_ops)
    WITH (lists = 100);

-- Update comments
COMMENT ON COLUMN members.embedding IS
'Vector embedding of searchable_text for semantic matching (1024 dimensions from voyage-3-large)';

COMMENT ON COLUMN organization_needs.embedding IS
'Vector embedding of description for semantic matching (1024 dimensions from voyage-3-large)';
