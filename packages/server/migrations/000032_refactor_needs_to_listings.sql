-- AGGRESSIVE REFACTOR: organization_needs → listings + type-specific tables
-- Safe because: project not launched yet

-- Step 1: Rename organization_needs → listings
ALTER TABLE organization_needs RENAME TO listings;

-- Step 2: Add new required fields for hybrid approach
ALTER TABLE listings
  ADD COLUMN IF NOT EXISTS listing_type TEXT NOT NULL DEFAULT 'service' CHECK (listing_type IN ('service', 'opportunity', 'business')),
  ADD COLUMN IF NOT EXISTS category TEXT NOT NULL DEFAULT 'other',
  ADD COLUMN IF NOT EXISTS capacity_status TEXT DEFAULT 'accepting' CHECK (capacity_status IN ('accepting', 'paused', 'at_capacity')),
  ADD COLUMN IF NOT EXISTS verified_at TIMESTAMPTZ,
  ADD COLUMN IF NOT EXISTS submitted_by_admin_id UUID,
  ADD COLUMN IF NOT EXISTS source_language TEXT NOT NULL DEFAULT 'en',
  ADD COLUMN IF NOT EXISTS location TEXT,
  ADD COLUMN IF NOT EXISTS latitude FLOAT,
  ADD COLUMN IF NOT EXISTS longitude FLOAT;

-- Update submission_type constraint if it exists
DO $$
BEGIN
  IF EXISTS (
    SELECT 1 FROM information_schema.columns
    WHERE table_name = 'listings' AND column_name = 'submission_type'
  ) THEN
    ALTER TABLE listings DROP CONSTRAINT IF EXISTS listings_submission_type_check;
    ALTER TABLE listings ADD CONSTRAINT listings_submission_type_check
      CHECK (submission_type IN ('scraped', 'admin', 'org_submitted'));
  ELSE
    ALTER TABLE listings ADD COLUMN submission_type TEXT DEFAULT 'scraped'
      CHECK (submission_type IN ('scraped', 'admin', 'org_submitted'));
  END IF;
END $$;

-- Step 3: Add organization_id column (links to organizations table)
ALTER TABLE listings
  ADD COLUMN IF NOT EXISTS organization_id UUID REFERENCES organizations(id);

-- Populate organization_id from organization_name
UPDATE listings l
SET organization_id = o.id
FROM organizations o
WHERE l.organization_name = o.name;

-- Step 4: Update urgency to match new enum
ALTER TABLE listings
  DROP CONSTRAINT IF EXISTS listings_urgency_check;

ALTER TABLE listings
  ADD CONSTRAINT listings_urgency_check
  CHECK (urgency IN ('low', 'medium', 'high', 'urgent'));

-- Step 5: Update status enum to match new values
UPDATE listings SET status = 'active' WHERE status = 'approved';

ALTER TABLE listings
  DROP CONSTRAINT IF EXISTS listings_status_check;

ALTER TABLE listings
  ADD CONSTRAINT listings_status_check
  CHECK (status IN ('pending_approval', 'active', 'filled', 'rejected', 'expired'));

-- Step 6: Create indexes for hot path queries
CREATE INDEX idx_listings_type ON listings(listing_type);
CREATE INDEX idx_listings_category ON listings(category);
CREATE INDEX idx_listings_capacity ON listings(capacity_status);
CREATE INDEX idx_listings_urgency ON listings(urgency) WHERE urgency IS NOT NULL;
CREATE INDEX idx_listings_verified ON listings(verified_at) WHERE verified_at IS NOT NULL;
CREATE INDEX idx_listings_organization ON listings(organization_id);
CREATE INDEX idx_listings_language ON listings(source_language);

-- Step 7: Create service-specific properties table
CREATE TABLE service_listings (
  listing_id UUID PRIMARY KEY REFERENCES listings(id) ON DELETE CASCADE,
  requires_id BOOL DEFAULT false,
  contacts_authorities BOOL DEFAULT false,
  avoids_facility_visit BOOL DEFAULT false,
  remote_ok BOOL DEFAULT false
);

COMMENT ON TABLE service_listings IS 'Service-specific properties (fear constraints for hot path)';

-- Populate service_listings for existing listings
INSERT INTO service_listings (listing_id)
SELECT id FROM listings WHERE listing_type = 'service';

-- Step 8: Create opportunity-specific properties table
CREATE TABLE opportunity_listings (
  listing_id UUID PRIMARY KEY REFERENCES listings(id) ON DELETE CASCADE,
  opportunity_type TEXT NOT NULL DEFAULT 'other' CHECK (opportunity_type IN ('volunteer', 'donation', 'customer', 'partnership', 'other')),
  time_commitment TEXT,
  requires_background_check BOOL DEFAULT false,
  minimum_age INT,
  skills_needed TEXT[],
  remote_ok BOOL DEFAULT false
);

COMMENT ON TABLE opportunity_listings IS 'Opportunity-specific properties (volunteer, donation, etc.)';

-- Step 9: Create business-specific properties table
CREATE TABLE business_listings (
  listing_id UUID PRIMARY KEY REFERENCES listings(id) ON DELETE CASCADE,
  business_type TEXT,
  support_needed TEXT[],
  current_situation TEXT,
  accepts_donations BOOL DEFAULT false,
  donation_link TEXT,
  gift_cards_available BOOL DEFAULT false,
  gift_card_link TEXT,
  remote_ok BOOL DEFAULT false,
  delivery_available BOOL DEFAULT false,
  online_ordering_link TEXT
);

CREATE INDEX idx_business_support_needed ON business_listings USING GIN(support_needed);

COMMENT ON TABLE business_listings IS 'Business-specific properties (economic solidarity)';

-- Step 10: Create delivery modes table (replacing simple fields)
CREATE TABLE listing_delivery_modes (
  listing_id UUID NOT NULL REFERENCES listings(id) ON DELETE CASCADE,
  delivery_mode TEXT NOT NULL CHECK (delivery_mode IN ('in_person', 'phone', 'online', 'mail', 'home_visit')),
  PRIMARY KEY (listing_id, delivery_mode)
);

COMMENT ON TABLE listing_delivery_modes IS 'How services can be accessed (multiple modes possible)';

-- Default all existing listings to in_person
INSERT INTO listing_delivery_modes (listing_id, delivery_mode)
SELECT id, 'in_person' FROM listings;

-- Step 11: Create contacts table (replacing JSONB contact_info)
CREATE TABLE listing_contacts (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  listing_id UUID NOT NULL REFERENCES listings(id) ON DELETE CASCADE,
  contact_type TEXT NOT NULL CHECK (contact_type IN ('phone', 'email', 'website', 'address')),
  contact_value TEXT NOT NULL,
  contact_label TEXT,
  display_order INT DEFAULT 0
);

CREATE INDEX idx_listing_contacts_listing ON listing_contacts(listing_id);

-- Migrate JSONB contact_info to structured table
INSERT INTO listing_contacts (listing_id, contact_type, contact_value)
SELECT
  id,
  'phone',
  contact_info->>'phone'
FROM listings
WHERE contact_info->>'phone' IS NOT NULL;

INSERT INTO listing_contacts (listing_id, contact_type, contact_value)
SELECT
  id,
  'email',
  contact_info->>'email'
FROM listings
WHERE contact_info->>'email' IS NOT NULL;

INSERT INTO listing_contacts (listing_id, contact_type, contact_value)
SELECT
  id,
  'website',
  contact_info->>'website'
FROM listings
WHERE contact_info->>'website' IS NOT NULL;

-- Step 12: Drop old JSONB contact_info column
ALTER TABLE listings DROP COLUMN contact_info;

-- Step 13: Rename old indexes
DROP INDEX IF EXISTS idx_organization_needs_status;
DROP INDEX IF EXISTS idx_organization_needs_content_hash;
DROP INDEX IF EXISTS idx_organization_needs_source_id;
DROP INDEX IF EXISTS idx_organization_needs_last_seen;

-- Step 14: Update comments
COMMENT ON TABLE listings IS 'Base listings table (services, opportunities, businesses)';
COMMENT ON COLUMN listings.listing_type IS 'Hot path: service/opportunity/business (affects routing)';
COMMENT ON COLUMN listings.category IS 'Hot path: primary navigation filter (food, housing, healthcare, legal)';
COMMENT ON COLUMN listings.capacity_status IS 'Hot path: visibility filter (accepting/paused/at_capacity)';
