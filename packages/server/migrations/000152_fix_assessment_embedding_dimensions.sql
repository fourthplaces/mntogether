-- Fix website_assessments embedding dimension from 1536 to 1024
-- The embedding service (text-embedding-3-small) produces 1024-dimensional vectors,
-- but this column was created with 1536 dimensions in migration 000063.

-- Drop and recreate with correct dimensions (existing embeddings are invalid anyway)
ALTER TABLE website_assessments DROP COLUMN IF EXISTS embedding;
ALTER TABLE website_assessments ADD COLUMN embedding vector(1024);

-- Recreate the index with correct dimensions
DROP INDEX IF EXISTS idx_website_assessments_embedding;
CREATE INDEX idx_website_assessments_embedding
    ON website_assessments USING ivfflat (embedding vector_cosine_ops)
    WITH (lists = 100);

-- Recreate the search function with correct dimensions
DROP FUNCTION IF EXISTS search_websites_by_similarity(vector(1536), float, int);

CREATE OR REPLACE FUNCTION search_websites_by_similarity(
    query_embedding vector(1024),
    match_threshold float DEFAULT 0.6,
    match_count int DEFAULT 20
)
RETURNS TABLE (
    website_id UUID,
    assessment_id UUID,
    website_url TEXT,
    organization_name TEXT,
    recommendation TEXT,
    assessment_markdown TEXT,
    similarity float
)
LANGUAGE plpgsql
AS $$
BEGIN
    RETURN QUERY
    SELECT
        w.id as website_id,
        wa.id as assessment_id,
        w.url as website_url,
        wa.organization_name,
        wa.recommendation,
        wa.assessment_markdown,
        (1 - (wa.embedding <=> query_embedding))::float as similarity
    FROM website_assessments wa
    JOIN websites w ON w.id = wa.website_id
    WHERE wa.embedding IS NOT NULL
        AND 1 - (wa.embedding <=> query_embedding) > match_threshold
    ORDER BY wa.embedding <=> query_embedding
    LIMIT match_count;
END;
$$;

COMMENT ON COLUMN website_assessments.embedding IS 'Semantic embedding (1024 dimensions from text-embedding-3-small)';
