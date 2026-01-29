-- Upgrade Vector Indexes to HNSW for 100K+ Scalability
--
-- PERFORMANCE: This migration replaces IVFFlat indexes with HNSW (Hierarchical Navigable Small World)
-- indexes for better performance at scale.
--
-- Performance comparison:
-- - IVFFlat with 500 lists: Good for 10K-50K records, 50-200ms search
-- - HNSW with m=16, ef_construction=64: Good for 100K-1M records, 10-50ms search
--
-- IMPORTANT: This migration may take several minutes on large datasets.
-- Run during low-traffic period. REINDEX operations lock the table.

-- Upgrade members embedding index
DROP INDEX IF EXISTS idx_members_embedding;
CREATE INDEX idx_members_embedding ON members
    USING hnsw (embedding vector_cosine_ops)
    WITH (m = 16, ef_construction = 64);

-- Upgrade organization_needs embedding index
DROP INDEX IF EXISTS idx_organization_needs_embedding;
CREATE INDEX idx_organization_needs_embedding ON organization_needs
    USING hnsw (embedding vector_cosine_ops)
    WITH (m = 16, ef_construction = 64);

-- Upgrade organizations embedding index
DROP INDEX IF EXISTS idx_organizations_embedding;
CREATE INDEX idx_organizations_embedding ON organizations
    USING hnsw (embedding vector_cosine_ops)
    WITH (m = 16, ef_construction = 64);

-- Add comments documenting the index parameters
COMMENT ON INDEX idx_members_embedding IS
    'HNSW index with m=16 (neighbors per layer), ef_construction=64 (build quality). Optimized for 100K-1M records.';

COMMENT ON INDEX idx_organization_needs_embedding IS
    'HNSW index with m=16 (neighbors per layer), ef_construction=64 (build quality). Optimized for 100K-1M records.';

COMMENT ON INDEX idx_organizations_embedding IS
    'HNSW index with m=16 (neighbors per layer), ef_construction=64 (build quality). Optimized for 100K-1M records.';

-- Performance tuning notes:
-- - m: Number of neighbors per layer (16 is optimal for most use cases)
-- - ef_construction: Higher = better recall but slower build (64 is balanced)
-- - At query time, can tune ef_search for speed/accuracy tradeoff
