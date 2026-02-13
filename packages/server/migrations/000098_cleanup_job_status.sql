-- Cleanup: drop the job_status enum type if it was partially created
-- The jobs table uses TEXT for status, not an enum
DROP TYPE IF EXISTS job_status CASCADE;

-- Ensure the status column is TEXT (it should already be)
-- This is a no-op if already TEXT
DO $$
BEGIN
    IF EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_name = 'jobs' AND column_name = 'status' AND udt_name = 'job_status'
    ) THEN
        ALTER TABLE jobs ALTER COLUMN status TYPE TEXT;
    END IF;
END $$;

-- Recreate the index (it may have been dropped)
DROP INDEX IF EXISTS idx_jobs_status_next_run;
CREATE INDEX idx_jobs_status_next_run ON jobs(status, next_run_at) WHERE status = 'pending';
