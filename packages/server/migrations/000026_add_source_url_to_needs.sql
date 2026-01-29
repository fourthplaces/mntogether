-- Add source_url column to organization_needs
-- Stores the specific page URL where each need was scraped from

ALTER TABLE organization_needs
ADD COLUMN source_url TEXT;

-- Add index for querying by source URL
CREATE INDEX idx_organization_needs_source_url ON organization_needs(source_url);

-- Comment for documentation
COMMENT ON COLUMN organization_needs.source_url IS 'The specific page URL where this need was scraped from (may be different from the main organization source URL)';
