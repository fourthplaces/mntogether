-- Organization sources (websites we monitor for needs)

CREATE TABLE organization_sources (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    -- Organization info
    organization_name TEXT NOT NULL,
    source_url TEXT NOT NULL UNIQUE,

    -- Scraping schedule
    last_scraped_at TIMESTAMPTZ,
    scrape_frequency_hours INTEGER DEFAULT 24,

    -- Status
    active BOOLEAN DEFAULT true,

    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Index for active sources
CREATE INDEX idx_organization_sources_active
    ON organization_sources(active)
    WHERE active = true;

-- Index for scheduling (find sources due for scraping)
CREATE INDEX idx_organization_sources_scrape_due
    ON organization_sources(last_scraped_at, scrape_frequency_hours)
    WHERE active = true;
