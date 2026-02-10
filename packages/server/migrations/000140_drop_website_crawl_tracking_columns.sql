-- Drop unused crawl tracking columns from websites table.
-- These were added in 000065 but never used by the crawl workflow.

DROP INDEX IF EXISTS idx_websites_crawl_status;

ALTER TABLE websites
    DROP COLUMN IF EXISTS crawl_status,
    DROP COLUMN IF EXISTS crawl_attempt_count,
    DROP COLUMN IF EXISTS max_crawl_retries,
    DROP COLUMN IF EXISTS last_crawl_started_at,
    DROP COLUMN IF EXISTS last_crawl_completed_at,
    DROP COLUMN IF EXISTS pages_crawled_count,
    DROP COLUMN IF EXISTS max_pages_per_crawl;
