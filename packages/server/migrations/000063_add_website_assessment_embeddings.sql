-- Add vector embeddings to website_assessments for semantic search
-- Enables finding websites by queries like "find me law firms"

-- Step 1: Add embedding column to website_assessments
ALTER TABLE website_assessments
    ADD COLUMN IF NOT EXISTS embedding vector(1536);

-- Step 2: Create vector similarity index
CREATE INDEX IF NOT EXISTS idx_website_assessments_embedding
    ON website_assessments USING ivfflat (embedding vector_cosine_ops)
    WITH (lists = 100);

-- Step 3: Add function to search websites by semantic similarity
CREATE OR REPLACE FUNCTION search_websites_by_similarity(
    query_embedding vector(1536),
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
        1 - (wa.embedding <=> query_embedding) as similarity
    FROM website_assessments wa
    JOIN websites w ON w.id = wa.website_id
    WHERE wa.embedding IS NOT NULL
        AND 1 - (wa.embedding <=> query_embedding) > match_threshold
    ORDER BY wa.embedding <=> query_embedding
    LIMIT match_count;
END;
$$;

COMMENT ON COLUMN website_assessments.embedding IS 'Vector embedding of assessment markdown for semantic search';
COMMENT ON FUNCTION search_websites_by_similarity IS 'Find websites semantically similar to query embedding';
