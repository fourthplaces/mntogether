-- Add scrape_urls array to organization_sources
-- If scrape_urls is set, only scrape those specific URLs
-- If null/empty, crawl the whole site from source_url

ALTER TABLE organization_sources
ADD COLUMN scrape_urls JSONB;

-- Comment for documentation
COMMENT ON COLUMN organization_sources.scrape_urls IS 'Optional array of specific URLs to scrape. If set, only these URLs will be scraped instead of crawling the whole site.';

-- Index for querying sources with specific URLs
CREATE INDEX idx_organization_sources_scrape_urls ON organization_sources USING gin(scrape_urls);
