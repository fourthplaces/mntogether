-- Junction table: Links domains to their submitted/scraped pages
-- Leverages existing page_snapshots table for content caching
CREATE TABLE IF NOT EXISTS domain_snapshots (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    domain_id UUID NOT NULL REFERENCES domains(id) ON DELETE CASCADE,
    page_url TEXT NOT NULL,
    page_snapshot_id UUID REFERENCES page_snapshots(id) ON DELETE SET NULL,
    submitted_by UUID REFERENCES members(id) ON DELETE SET NULL,
    submitted_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_scraped_at TIMESTAMPTZ,
    scrape_status TEXT NOT NULL DEFAULT 'pending' CHECK (scrape_status IN ('pending', 'scraped', 'failed')),
    scrape_error TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(domain_id, page_url)
);

CREATE INDEX IF NOT EXISTS idx_domain_snapshots_domain_id ON domain_snapshots(domain_id);
CREATE INDEX IF NOT EXISTS idx_domain_snapshots_page_snapshot_id ON domain_snapshots(page_snapshot_id);
CREATE INDEX IF NOT EXISTS idx_domain_snapshots_scrape_status ON domain_snapshots(scrape_status);
CREATE INDEX IF NOT EXISTS idx_domain_snapshots_pending ON domain_snapshots(domain_id, scrape_status) WHERE scrape_status = 'pending';

-- Add index for finding listings by domain (for hiding when domain suspended)
CREATE INDEX IF NOT EXISTS idx_listings_domain_id ON listings(domain_id) WHERE domain_id IS NOT NULL;
