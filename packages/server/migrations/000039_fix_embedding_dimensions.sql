-- Fix Embedding Dimension Mismatch
--
-- CRITICAL: Migration 000022 changed embeddings to 1024 dimensions for members and needs,
-- but migration 000038 added 1536 dimensions for organizations. This creates inconsistency
-- and will cause semantic search failures.
--
-- This migration standardizes all embeddings to 1024 dimensions to match the embedding
-- service configuration (text-embedding-3-small produces 1024-dimensional vectors).

-- Fix organizations table embedding dimension
ALTER TABLE organizations DROP COLUMN IF EXISTS embedding;
ALTER TABLE organizations ADD COLUMN embedding vector(1024);

-- Update the search function to use correct dimensions
DROP FUNCTION IF EXISTS search_organizations_by_similarity(vector(1536), double precision, integer);

CREATE OR REPLACE FUNCTION search_organizations_by_similarity(
  query_embedding vector(1024),
  threshold double precision DEFAULT 0.7,
  result_limit integer DEFAULT 10
)
RETURNS TABLE (
  id UUID,
  organization_name TEXT,
  description TEXT,
  similarity double precision
) AS $$
BEGIN
  RETURN QUERY
  SELECT
    o.id,
    o.name as organization_name,
    o.description,
    1 - (o.embedding <=> query_embedding) as similarity
  FROM organizations o
  WHERE o.embedding IS NOT NULL
    AND (1 - (o.embedding <=> query_embedding)) >= threshold
  ORDER BY o.embedding <=> query_embedding
  LIMIT result_limit;
END;
$$ LANGUAGE plpgsql IMMUTABLE;

-- Add comment documenting the dimension standard
COMMENT ON COLUMN organizations.embedding IS 'Semantic embedding (1024 dimensions from text-embedding-3-small)';
COMMENT ON COLUMN members.embedding IS 'Semantic embedding (1024 dimensions from text-embedding-3-small)';
COMMENT ON COLUMN listings.embedding IS 'Semantic embedding (1024 dimensions from text-embedding-3-small)';
