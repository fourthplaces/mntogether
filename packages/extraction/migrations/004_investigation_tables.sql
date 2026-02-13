-- Migration 004: Detective loop investigation tracking infrastructure
-- Pure mechanical tracking tables - no policy enforcement
--
-- Design Principle: "Mechanism, not Policy"
-- - Tables track WHAT happened (observations)
-- - Application controls WHEN to stop (policy decisions)
-- - Token budgets, iteration limits, retry thresholds belong in caller's orchestrator

-- Grounding grade enum (matches Rust GroundingGrade)
DO $$ BEGIN
    CREATE TYPE grounding_grade AS ENUM ('verified', 'single_source', 'conflicted', 'inferred');
EXCEPTION
    WHEN duplicate_object THEN NULL;
END $$;

-- =============================================================================
-- Extraction Jobs: Tracks extraction requests and their outcomes
-- =============================================================================
CREATE TABLE IF NOT EXISTS extraction_jobs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    -- Query information
    query TEXT NOT NULL,
    query_hash TEXT NOT NULL,           -- For caching/deduplication

    -- Strategy used (matches ExtractionStrategy enum)
    strategy TEXT NOT NULL CHECK (strategy IN ('collection', 'singular', 'narrative')),

    -- Outcome tracking (mechanical observations)
    grounding grounding_grade,
    has_gaps BOOLEAN DEFAULT FALSE,
    tokens_used INTEGER DEFAULT 0,      -- Observation of what happened

    -- Timestamps
    started_at TIMESTAMPTZ DEFAULT NOW(),
    completed_at TIMESTAMPTZ,
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

-- Query hash for cache lookups
CREATE INDEX IF NOT EXISTS idx_extraction_jobs_query_hash
    ON extraction_jobs(query_hash);

-- Find incomplete jobs
CREATE INDEX IF NOT EXISTS idx_extraction_jobs_incomplete
    ON extraction_jobs(started_at) WHERE completed_at IS NULL;

-- =============================================================================
-- Extraction Gaps: Tracks gaps detected in extractions
-- =============================================================================
CREATE TABLE IF NOT EXISTS extraction_gaps (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    -- Job relationship
    job_id UUID NOT NULL REFERENCES extraction_jobs(id) ON DELETE CASCADE,

    -- Lineage tracking (mechanical - for queries, not enforcement)
    parent_gap_id UUID REFERENCES extraction_gaps(id) ON DELETE SET NULL,
    depth INTEGER DEFAULT 0,            -- Investigation depth (query convenience)

    -- Gap information
    field TEXT NOT NULL,                -- Human-readable field name
    query TEXT NOT NULL,                -- Search query for resolution

    -- Gap classification (for weight tuning in hybrid search)
    gap_type TEXT CHECK (gap_type IN ('entity', 'semantic', 'structural')),

    -- Status (mechanical state, not policy)
    status TEXT DEFAULT 'pending' CHECK (status IN ('pending', 'investigating', 'resolved', 'abandoned')),

    -- Resolution tracking
    resolved_at TIMESTAMPTZ,
    resolution_source TEXT,             -- URL or method that resolved the gap

    -- Garbage collection support (Gemini feedback)
    -- Gaps expire if not updated - prevents orphaned gaps from failed jobs
    expires_at TIMESTAMPTZ DEFAULT NOW() + INTERVAL '24 hours',
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Find gaps by job
CREATE INDEX IF NOT EXISTS idx_extraction_gaps_job
    ON extraction_gaps(job_id);

-- Lineage queries (find children of a gap)
CREATE INDEX IF NOT EXISTS idx_extraction_gaps_parent
    ON extraction_gaps(parent_gap_id) WHERE parent_gap_id IS NOT NULL;

-- Find pending gaps (for orchestrator to process)
CREATE INDEX IF NOT EXISTS idx_extraction_gaps_pending
    ON extraction_gaps(job_id, status) WHERE status = 'pending';

-- Depth-based queries (find gaps at specific investigation depth)
CREATE INDEX IF NOT EXISTS idx_extraction_gaps_depth
    ON extraction_gaps(job_id, depth);

-- Garbage collection: find expired gaps
CREATE INDEX IF NOT EXISTS idx_extraction_gaps_expired
    ON extraction_gaps(expires_at) WHERE status IN ('pending', 'investigating');

-- =============================================================================
-- Investigation Logs: Audit trail of investigation attempts
-- Pure observation - records what was tried, not what should happen
-- =============================================================================
CREATE TABLE IF NOT EXISTS extraction_investigation_logs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    -- Gap being investigated
    gap_id UUID NOT NULL REFERENCES extraction_gaps(id) ON DELETE CASCADE,

    -- What action was taken
    action_type TEXT NOT NULL,          -- 'hybrid_search', 'fetch_url', 'crawl_site'

    -- Action parameters (for replay/debugging)
    action_params JSONB NOT NULL DEFAULT '{}',  -- { "query": "...", "semantic_weight": 0.3 }

    -- Outcome observation
    pages_found INTEGER DEFAULT 0,
    tokens_used INTEGER DEFAULT 0,

    -- Timing
    executed_at TIMESTAMPTZ DEFAULT NOW(),
    duration_ms INTEGER                 -- How long the action took
);

-- Find logs for a gap
CREATE INDEX IF NOT EXISTS idx_investigation_logs_gap
    ON extraction_investigation_logs(gap_id);

-- Find recent investigations (for debugging)
CREATE INDEX IF NOT EXISTS idx_investigation_logs_recent
    ON extraction_investigation_logs(executed_at DESC);

-- =============================================================================
-- Gap Cache: Mechanical optimization for repeated gap queries
-- =============================================================================
CREATE TABLE IF NOT EXISTS extraction_gap_cache (
    query_hash TEXT PRIMARY KEY,
    results JSONB NOT NULL,             -- Cached page references
    created_at TIMESTAMPTZ DEFAULT NOW(),
    expires_at TIMESTAMPTZ DEFAULT NOW() + INTERVAL '1 hour'
);

-- Cleanup expired cache entries
CREATE INDEX IF NOT EXISTS idx_gap_cache_expired
    ON extraction_gap_cache(expires_at);

-- =============================================================================
-- Helper Functions
-- =============================================================================

-- Function to clean up expired gaps (call periodically)
CREATE OR REPLACE FUNCTION cleanup_expired_gaps()
RETURNS INTEGER AS $$
DECLARE
    deleted_count INTEGER;
BEGIN
    -- Mark expired pending/investigating gaps as abandoned
    UPDATE extraction_gaps
    SET status = 'abandoned'
    WHERE expires_at < NOW()
      AND status IN ('pending', 'investigating');

    GET DIAGNOSTICS deleted_count = ROW_COUNT;

    -- Clean up expired cache
    DELETE FROM extraction_gap_cache WHERE expires_at < NOW();

    RETURN deleted_count;
END;
$$ LANGUAGE plpgsql;

-- Function to extend gap expiration (call when actively investigating)
CREATE OR REPLACE FUNCTION extend_gap_expiration(gap_id_param UUID, hours INTEGER DEFAULT 24)
RETURNS VOID AS $$
BEGIN
    UPDATE extraction_gaps
    SET expires_at = NOW() + (hours || ' hours')::INTERVAL,
        updated_at = NOW()
    WHERE id = gap_id_param;
END;
$$ LANGUAGE plpgsql;

-- =============================================================================
-- Trigger: Update timestamps
-- =============================================================================
CREATE OR REPLACE FUNCTION update_job_timestamp()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at := NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trg_update_job_timestamp ON extraction_jobs;
CREATE TRIGGER trg_update_job_timestamp
    BEFORE UPDATE ON extraction_jobs
    FOR EACH ROW
    EXECUTE FUNCTION update_job_timestamp();
