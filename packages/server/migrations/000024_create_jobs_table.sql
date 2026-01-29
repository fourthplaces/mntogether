-- Create jobs table for background task processing
-- This table stores jobs for seesaw-rs event-driven architecture

CREATE TABLE IF NOT EXISTS jobs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    status TEXT NOT NULL DEFAULT 'pending',
    job_type TEXT NOT NULL,
    args JSONB NOT NULL,

    -- Scheduling
    next_run_at TIMESTAMP WITH TIME ZONE,
    last_run_at TIMESTAMP WITH TIME ZONE,

    -- Retry handling
    max_retries INTEGER NOT NULL DEFAULT 3,
    retry_count INTEGER NOT NULL DEFAULT 0,

    -- Versioning and idempotency
    version INTEGER NOT NULL DEFAULT 1,
    idempotency_key TEXT,

    -- Metadata
    reference_id UUID,
    priority INTEGER NOT NULL DEFAULT 0,

    -- Error tracking
    error_message TEXT,
    error_kind TEXT,

    -- Timestamps
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- Indexes for efficient job processing
CREATE INDEX idx_jobs_status_next_run ON jobs(status, next_run_at) WHERE status = 'pending';
CREATE INDEX idx_jobs_job_type ON jobs(job_type);
CREATE INDEX idx_jobs_reference_id ON jobs(reference_id) WHERE reference_id IS NOT NULL;
CREATE UNIQUE INDEX idx_jobs_idempotency_key ON jobs(idempotency_key)
    WHERE idempotency_key IS NOT NULL AND status IN ('pending', 'running');

-- Comments
COMMENT ON TABLE jobs IS 'Background job queue for seesaw-rs commands';
COMMENT ON COLUMN jobs.status IS 'Job status: pending, running, completed, failed, dead_letter';
COMMENT ON COLUMN jobs.job_type IS 'Type of job (e.g., scrape_resource_link, extract_needs)';
COMMENT ON COLUMN jobs.args IS 'Serialized command payload';
COMMENT ON COLUMN jobs.idempotency_key IS 'Prevents duplicate job execution';
