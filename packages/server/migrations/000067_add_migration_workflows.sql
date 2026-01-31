-- Migration workflow tracking table for data migrations
-- Tracks state, progress, and cursor position for resumable migrations

CREATE TABLE migration_workflows (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name TEXT NOT NULL UNIQUE,
    phase TEXT NOT NULL DEFAULT 'running',  -- running, paused, completed, failed
    total_items BIGINT NOT NULL DEFAULT 0,
    completed_items BIGINT NOT NULL DEFAULT 0,
    failed_items BIGINT NOT NULL DEFAULT 0,
    skipped_items BIGINT NOT NULL DEFAULT 0,
    last_processed_id UUID,
    dry_run BOOLEAN NOT NULL DEFAULT true,
    error_budget DECIMAL(5,4) NOT NULL DEFAULT 0.01,
    started_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    paused_at TIMESTAMPTZ,
    completed_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Index for looking up workflow by name (already unique, but explicit index for clarity)
CREATE INDEX idx_migration_workflows_name ON migration_workflows(name);

-- Index for finding active workflows
CREATE INDEX idx_migration_workflows_phase ON migration_workflows(phase) WHERE phase IN ('running', 'paused');
