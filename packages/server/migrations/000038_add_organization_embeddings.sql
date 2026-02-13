-- Add vector embeddings to organizations for semantic search
-- Enables AI-powered matching in chat and referral documents

-- Step 1: Add embedding column to organizations
ALTER TABLE organizations
  ADD COLUMN IF NOT EXISTS embedding vector(1536);

-- Step 2: Add summary for rich service info (used with description for embeddings)
ALTER TABLE organizations
  ADD COLUMN IF NOT EXISTS summary TEXT;

-- Step 3: Create vector similarity index
CREATE INDEX IF NOT EXISTS idx_organizations_embedding
  ON organizations USING ivfflat (embedding vector_cosine_ops)
  WITH (lists = 100);

-- Step 4: Add function to search organizations by semantic similarity
CREATE OR REPLACE FUNCTION search_organizations_by_similarity(
  query_embedding vector(1536),
  match_threshold float DEFAULT 0.7,
  match_count int DEFAULT 10
)
RETURNS TABLE (
  id UUID,
  name TEXT,
  description TEXT,
  summary TEXT,
  similarity float
)
LANGUAGE plpgsql
AS $$
BEGIN
  RETURN QUERY
  SELECT
    o.id,
    o.name,
    o.description,
    o.summary,
    1 - (o.embedding <=> query_embedding) as similarity
  FROM organizations o
  WHERE o.embedding IS NOT NULL
    AND 1 - (o.embedding <=> query_embedding) > match_threshold
  ORDER BY o.embedding <=> query_embedding
  LIMIT match_count;
END;
$$;

COMMENT ON COLUMN organizations.embedding IS 'Vector embedding of organization (description + summary) for semantic search';
COMMENT ON COLUMN organizations.summary IS 'Rich summary of all services, specialties, and capabilities for AI matching';
COMMENT ON FUNCTION search_organizations_by_similarity IS 'Find organizations semantically similar to query embedding (used in chat & referrals)';
