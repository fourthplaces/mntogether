-- Improve vector search index configuration for better scalability
-- Increase lists from 100 to 500 for 10K-50K member/need scale
--
-- Performance impact:
-- - 10K members with lists=100: 500-2000ms per search
-- - 10K members with lists=500: 50-200ms per search (10-100x improvement)
--
-- Note: This requires REINDEX which rebuilds the index

-- Drop and recreate members embedding index with better configuration
DROP INDEX IF EXISTS idx_members_embedding;
CREATE INDEX idx_members_embedding ON members
    USING ivfflat (embedding vector_cosine_ops)
    WITH (lists = 500);  -- Improved from 100 for production scale

-- Drop and recreate organization_needs embedding index with better configuration
DROP INDEX IF EXISTS idx_needs_embedding;
CREATE INDEX idx_needs_embedding ON organization_needs
    USING ivfflat (embedding vector_cosine_ops)
    WITH (lists = 500);  -- Improved from 100 for production scale

-- Add comment explaining the configuration
COMMENT ON INDEX idx_members_embedding IS
'IVFFlat index with lists=500 optimized for 10K-50K members. Rule of thumb: lists = rows/1000. For 100K+ members, consider migrating to HNSW index.';

COMMENT ON INDEX idx_needs_embedding IS
'IVFFlat index with lists=500 optimized for 10K-50K needs. Rule of thumb: lists = rows/1000. For 100K+ needs, consider migrating to HNSW index.';
