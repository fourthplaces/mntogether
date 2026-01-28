-- Scrape jobs table for async scraping workflow
-- Allows GraphQL to return immediately with job_id instead of blocking

CREATE TYPE scrape_job_status AS ENUM ('pending', 'scraping', 'extracting', 'syncing', 'completed', 'failed');

CREATE TABLE scrape_jobs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    source_id UUID NOT NULL REFERENCES organization_sources(id) ON DELETE CASCADE,

    -- Status tracking
    status scrape_job_status NOT NULL DEFAULT 'pending',
    error_message TEXT,

    -- Progress tracking
    scraped_at TIMESTAMPTZ,
    extracted_at TIMESTAMPTZ,
    synced_at TIMESTAMPTZ,

    -- Results (populated when completed)
    new_needs_count INTEGER,
    changed_needs_count INTEGER,
    disappeared_needs_count INTEGER,

    -- Timestamps
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at TIMESTAMPTZ
);

-- Index for finding active jobs
CREATE INDEX idx_scrape_jobs_status ON scrape_jobs(status);

-- Index for source lookups
CREATE INDEX idx_scrape_jobs_source_id ON scrape_jobs(source_id);

-- Trigger to update updated_at
CREATE OR REPLACE FUNCTION update_scrape_jobs_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trigger_update_scrape_jobs_updated_at
    BEFORE UPDATE ON scrape_jobs
    FOR EACH ROW
    EXECUTE FUNCTION update_scrape_jobs_updated_at();

COMMENT ON TABLE scrape_jobs IS
'Tracks async scraping jobs. GraphQL returns job_id immediately, admin polls for progress.';
