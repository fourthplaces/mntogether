-- AGGRESSIVE REFACTOR: organization_sources → domains + organizations
-- Safe because: project not launched yet

-- Step 1: Create new domains table
CREATE TABLE domains (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  domain_url TEXT NOT NULL UNIQUE,
  scrape_frequency_hours INT DEFAULT 24,
  last_scraped_at TIMESTAMPTZ,
  active BOOL DEFAULT true,
  created_at TIMESTAMPTZ DEFAULT NOW(),
  updated_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_domains_active ON domains(active);

COMMENT ON TABLE domains IS 'Websites we scrape (e.g., nonprofit directories, government sites)';

-- Step 2: Create domain_scrape_urls for specific pages
CREATE TABLE domain_scrape_urls (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  domain_id UUID NOT NULL REFERENCES domains(id) ON DELETE CASCADE,
  url TEXT NOT NULL,
  active BOOL DEFAULT true,
  added_at TIMESTAMPTZ DEFAULT NOW(),
  UNIQUE(domain_id, url)
);

CREATE INDEX idx_domain_scrape_urls_domain ON domain_scrape_urls(domain_id);

-- Step 3: Drop and recreate organizations table with new schema
DROP TABLE IF EXISTS organizations CASCADE;

CREATE TABLE organizations (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  name TEXT NOT NULL UNIQUE,
  description TEXT,
  domain_id UUID REFERENCES domains(id),

  -- Contact
  website TEXT,
  phone TEXT,
  email TEXT,

  -- Location
  primary_address TEXT,
  latitude FLOAT,
  longitude FLOAT,

  -- Verification
  verified BOOL DEFAULT false,
  verified_at TIMESTAMPTZ,

  -- Claiming (for capacity updates)
  claimed_at TIMESTAMPTZ,
  claim_token TEXT UNIQUE,
  claim_email TEXT,

  organization_type TEXT CHECK (organization_type IN ('nonprofit', 'government', 'business', 'community', 'other')),

  created_at TIMESTAMPTZ DEFAULT NOW(),
  updated_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_organizations_name ON organizations(name);
CREATE INDEX idx_organizations_domain ON organizations(domain_id);
CREATE INDEX idx_organizations_verified ON organizations(verified);
CREATE INDEX idx_organizations_claimed ON organizations(claimed_at) WHERE claimed_at IS NOT NULL;

COMMENT ON TABLE organizations IS 'Organizations providing services or opportunities';
COMMENT ON COLUMN organizations.claimed_at IS 'When organization claimed their profile for self-service';

-- Step 4: Migrate data from organization_sources
-- Each source becomes: 1 domain + 1 organization
INSERT INTO domains (domain_url, scrape_frequency_hours, last_scraped_at, active, created_at)
SELECT
  source_url,
  scrape_frequency_hours,
  last_scraped_at,
  active,
  created_at
FROM organization_sources;

-- Create organizations from sources
INSERT INTO organizations (name, domain_id, created_at)
SELECT
  os.organization_name,
  d.id,
  os.created_at
FROM organization_sources os
JOIN domains d ON d.domain_url = os.source_url;

-- Step 5: Update organization_needs to reference domains instead of sources
ALTER TABLE organization_needs
  ADD COLUMN domain_id UUID REFERENCES domains(id);

-- Migrate source_id → domain_id
UPDATE organization_needs n
SET domain_id = d.id
FROM organization_sources os
JOIN domains d ON d.domain_url = os.source_url
WHERE n.source_id = os.id;

-- Step 6: Drop old source_id column
ALTER TABLE organization_needs DROP COLUMN source_id;

-- Step 7: Keep organization_sources for now (will drop in next migration after confirming data migration)
-- We'll do a gradual migration to be safe
