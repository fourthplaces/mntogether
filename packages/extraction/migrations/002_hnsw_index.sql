-- Migration 002: Upgrade from IVFFLAT to HNSW vector index
-- HNSW provides better recall at query time, critical for Detective recursive loops
--
-- Parameters chosen for 1M-10M scale (adjust for your dataset):
-- | Scale   | m  | ef_construction | ef_search | Query Time | Recall |
-- |---------|----|-----------------|-----------| -----------|--------|
-- | <1M     | 16 | 64              | 40        | <50ms      | 95%    |
-- | 1M-10M  | 24 | 128             | 60        | <80ms      | 97%    |
-- | 10M+    | 32 | 256             | 80        | <120ms     | 98%    |
--
-- Note: Higher ef_construction increases build time but improves Detective precision
-- where accurate pivot searches determine loop termination.

-- Requires pgvector 0.5.0+ for HNSW support
-- Check version first
DO $$
DECLARE
    pgv_version TEXT;
BEGIN
    SELECT extversion INTO pgv_version FROM pg_extension WHERE extname = 'vector';

    IF pgv_version IS NULL THEN
        RAISE NOTICE 'pgvector extension not installed. Skipping HNSW index creation.';
        RETURN;
    END IF;

    -- pgvector 0.5.0+ required for HNSW
    IF pgv_version < '0.5.0' THEN
        RAISE NOTICE 'pgvector % does not support HNSW. Version 0.5.0+ required.', pgv_version;
        RETURN;
    END IF;

    -- Drop the old IVFFLAT index if it exists
    DROP INDEX IF EXISTS idx_extraction_embeddings_vector;

    -- Create new HNSW index with Detective-optimized parameters
    -- m=24: connections per layer (higher = better recall, more memory)
    -- ef_construction=128: build-time search depth (higher = slower build, better index)
    EXECUTE '
        CREATE INDEX idx_extraction_embeddings_hnsw
        ON extraction_embeddings USING hnsw (embedding vector_cosine_ops)
        WITH (m = 24, ef_construction = 128)
    ';

    RAISE NOTICE 'HNSW index created successfully on pgvector %', pgv_version;
END $$;

-- Note: At query time, set hnsw.ef_search for precision vs speed tradeoff:
-- SET LOCAL hnsw.ef_search = 60;  -- 97% recall @ <80ms for 1M-10M scale
-- Default is 40 which is fine for most queries.
