-- Sync jobs table with Rust Job struct
-- Adds all missing columns that the Rust code expects

-- Scheduling columns
ALTER TABLE jobs ADD COLUMN IF NOT EXISTS frequency TEXT;
ALTER TABLE jobs ADD COLUMN IF NOT EXISTS timezone TEXT NOT NULL DEFAULT 'UTC';

-- Policy columns (stored as TEXT, converted in Rust)
ALTER TABLE jobs ADD COLUMN IF NOT EXISTS overlap_policy TEXT NOT NULL DEFAULT 'skip';
ALTER TABLE jobs ADD COLUMN IF NOT EXISTS misfire_policy TEXT NOT NULL DEFAULT 'skip_to_latest';

-- Execution settings
ALTER TABLE jobs ADD COLUMN IF NOT EXISTS timeout_ms BIGINT NOT NULL DEFAULT 300000;
ALTER TABLE jobs ADD COLUMN IF NOT EXISTS lease_duration_ms BIGINT NOT NULL DEFAULT 60000;

-- Lease management
ALTER TABLE jobs ADD COLUMN IF NOT EXISTS lease_expires_at TIMESTAMPTZ;
ALTER TABLE jobs ADD COLUMN IF NOT EXISTS worker_id TEXT;

-- State
ALTER TABLE jobs ADD COLUMN IF NOT EXISTS enabled BOOLEAN NOT NULL DEFAULT true;

-- Multi-tenancy
ALTER TABLE jobs ADD COLUMN IF NOT EXISTS container_id UUID;

-- Workflow coordination
ALTER TABLE jobs ADD COLUMN IF NOT EXISTS workflow_id UUID;

-- Dead letter workflow
ALTER TABLE jobs ADD COLUMN IF NOT EXISTS dead_lettered_at TIMESTAMPTZ;
ALTER TABLE jobs ADD COLUMN IF NOT EXISTS dead_letter_reason TEXT;
ALTER TABLE jobs ADD COLUMN IF NOT EXISTS replay_count INTEGER NOT NULL DEFAULT 0;
ALTER TABLE jobs ADD COLUMN IF NOT EXISTS resolved_at TIMESTAMPTZ;
ALTER TABLE jobs ADD COLUMN IF NOT EXISTS resolution_note TEXT;

-- Retry chain tracing
ALTER TABLE jobs ADD COLUMN IF NOT EXISTS root_job_id UUID;
ALTER TABLE jobs ADD COLUMN IF NOT EXISTS dedupe_key TEXT;
ALTER TABLE jobs ADD COLUMN IF NOT EXISTS attempt INTEGER NOT NULL DEFAULT 1;

-- Command-level idempotency
ALTER TABLE jobs ADD COLUMN IF NOT EXISTS command_version INTEGER NOT NULL DEFAULT 1;

-- Make args nullable (Rust has Option<Value>)
ALTER TABLE jobs ALTER COLUMN args DROP NOT NULL;

-- Add indexes for new columns
CREATE INDEX IF NOT EXISTS idx_jobs_workflow_id ON jobs(workflow_id) WHERE workflow_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_jobs_container_id ON jobs(container_id) WHERE container_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_jobs_enabled ON jobs(enabled) WHERE enabled = true;
CREATE INDEX IF NOT EXISTS idx_jobs_dead_letter ON jobs(status) WHERE status = 'dead_letter';

-- Add unique constraint for upsert (reference_id + job_type)
-- Drop if exists first to avoid errors
DROP INDEX IF EXISTS idx_jobs_reference_job_type_unique;
CREATE UNIQUE INDEX idx_jobs_reference_job_type_unique ON jobs(reference_id, job_type) WHERE reference_id IS NOT NULL;

-- Comments
COMMENT ON COLUMN jobs.frequency IS 'RRULE or cron expression for recurring jobs';
COMMENT ON COLUMN jobs.timezone IS 'Timezone for scheduling (default UTC)';
COMMENT ON COLUMN jobs.overlap_policy IS 'How to handle overlapping runs: allow, skip, coalesce_latest';
COMMENT ON COLUMN jobs.misfire_policy IS 'How to handle missed runs: catch_up, skip_to_latest';
COMMENT ON COLUMN jobs.timeout_ms IS 'Job execution timeout in milliseconds';
COMMENT ON COLUMN jobs.lease_duration_ms IS 'How long a worker can hold the job';
COMMENT ON COLUMN jobs.lease_expires_at IS 'When the current lease expires';
COMMENT ON COLUMN jobs.worker_id IS 'ID of worker currently processing this job';
COMMENT ON COLUMN jobs.enabled IS 'Whether the job is enabled for execution';
COMMENT ON COLUMN jobs.container_id IS 'Multi-tenancy container ID';
COMMENT ON COLUMN jobs.workflow_id IS 'Parent workflow ID for job coordination';
COMMENT ON COLUMN jobs.dead_lettered_at IS 'When job was moved to dead letter';
COMMENT ON COLUMN jobs.dead_letter_reason IS 'Why job was dead lettered';
COMMENT ON COLUMN jobs.replay_count IS 'Number of times job was replayed from dead letter';
COMMENT ON COLUMN jobs.resolved_at IS 'When dead letter was resolved';
COMMENT ON COLUMN jobs.resolution_note IS 'Notes about dead letter resolution';
COMMENT ON COLUMN jobs.root_job_id IS 'Original job ID in retry chain';
COMMENT ON COLUMN jobs.dedupe_key IS 'Key for deduplication';
COMMENT ON COLUMN jobs.attempt IS 'Current attempt number';
COMMENT ON COLUMN jobs.command_version IS 'Version of the command schema';
