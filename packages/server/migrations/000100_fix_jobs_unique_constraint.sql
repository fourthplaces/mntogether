-- Fix the unique constraint for ON CONFLICT to work properly
-- ON CONFLICT requires exact constraint match, partial indexes don't work

-- Drop the partial index
DROP INDEX IF EXISTS idx_jobs_reference_job_type_unique;

-- Create a proper unique constraint (not a partial index)
-- This allows ON CONFLICT (reference_id, job_type) to work
ALTER TABLE jobs ADD CONSTRAINT jobs_reference_id_job_type_unique
    UNIQUE (reference_id, job_type);
