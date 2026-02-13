-- Re-add vector embeddings to posts for semantic search
-- (Previously dropped in 000093 when used for deduplication)

-- Add embedding column (1024 dimensions per codebase standard)
ALTER TABLE posts ADD COLUMN IF NOT EXISTS embedding vector(1024);

-- Create HNSW index for efficient similarity search
CREATE INDEX IF NOT EXISTS idx_posts_embedding
    ON posts USING hnsw (embedding vector_cosine_ops)
    WITH (m = 16, ef_construction = 64);

-- Search function
CREATE OR REPLACE FUNCTION search_posts_by_similarity(
    query_embedding vector(1024),
    match_threshold float DEFAULT 0.6,
    match_count int DEFAULT 20
)
RETURNS TABLE (
    post_id UUID,
    title TEXT,
    description TEXT,
    organization_name TEXT,
    category TEXT,
    post_type TEXT,
    similarity float
)
LANGUAGE plpgsql AS $$
BEGIN
    RETURN QUERY
    SELECT
        p.id as post_id,
        p.title,
        p.description,
        p.organization_name,
        p.category,
        p.post_type,
        1 - (p.embedding <=> query_embedding) as similarity
    FROM posts p
    WHERE p.embedding IS NOT NULL
        AND p.deleted_at IS NULL
        AND p.status = 'active'
        AND 1 - (p.embedding <=> query_embedding) > match_threshold
    ORDER BY p.embedding <=> query_embedding
    LIMIT match_count;
END;
$$;

COMMENT ON COLUMN posts.embedding IS 'Semantic embedding (1024 dimensions) for search';
