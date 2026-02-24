-- Add embedding columns for vector similarity search
-- Uses pgvector extension (already enabled in 20260127000001)

-- Add embedding to members (for matching members to needs)
ALTER TABLE members
    ADD COLUMN embedding vector(1536);  -- text-embedding-3-small dimensions

-- Index for fast cosine similarity search
CREATE INDEX idx_members_embedding ON members
    USING ivfflat (embedding vector_cosine_ops)
    WITH (lists = 100);

COMMENT ON COLUMN members.embedding IS
'Vector embedding of searchable_text for semantic matching (1536 dimensions from text-embedding-3-small)';

-- Add embedding to organization_needs (for matching needs to members)
ALTER TABLE organization_needs
    ADD COLUMN embedding vector(1536);

-- Index for fast cosine similarity search
CREATE INDEX idx_needs_embedding ON organization_needs
    USING ivfflat (embedding vector_cosine_ops)
    WITH (lists = 100);

COMMENT ON COLUMN organization_needs.embedding IS
'Vector embedding of description for semantic matching (1536 dimensions from text-embedding-3-small)';
