-- Genericize service_listings table for universal service properties
-- Previously had immigration-specific fields, now supports all service types

-- Step 1: Rename immigration-specific columns to generic equivalents
ALTER TABLE service_listings
  RENAME COLUMN requires_id TO requires_identification;

ALTER TABLE service_listings
  RENAME COLUMN remote_ok TO remote_available;

-- Step 2: Drop immigration-specific columns that don't map to generic concepts
ALTER TABLE service_listings
  DROP COLUMN IF EXISTS contacts_authorities;

ALTER TABLE service_listings
  DROP COLUMN IF EXISTS avoids_facility_visit;

-- Step 3: Add new generic service properties
ALTER TABLE service_listings
  ADD COLUMN IF NOT EXISTS requires_appointment BOOL DEFAULT false,
  ADD COLUMN IF NOT EXISTS walk_ins_accepted BOOL DEFAULT true,
  ADD COLUMN IF NOT EXISTS in_person_available BOOL DEFAULT true,
  ADD COLUMN IF NOT EXISTS home_visits_available BOOL DEFAULT false,
  ADD COLUMN IF NOT EXISTS wheelchair_accessible BOOL DEFAULT false,
  ADD COLUMN IF NOT EXISTS interpretation_available BOOL DEFAULT false,
  ADD COLUMN IF NOT EXISTS free_service BOOL DEFAULT false,
  ADD COLUMN IF NOT EXISTS sliding_scale_fees BOOL DEFAULT false,
  ADD COLUMN IF NOT EXISTS accepts_insurance BOOL DEFAULT false,
  ADD COLUMN IF NOT EXISTS evening_hours BOOL DEFAULT false,
  ADD COLUMN IF NOT EXISTS weekend_hours BOOL DEFAULT false;

-- Step 4: Create indexes for commonly filtered properties
CREATE INDEX IF NOT EXISTS idx_service_listings_remote ON service_listings(remote_available) WHERE remote_available = true;
CREATE INDEX IF NOT EXISTS idx_service_listings_free ON service_listings(free_service) WHERE free_service = true;
CREATE INDEX IF NOT EXISTS idx_service_listings_wheelchair ON service_listings(wheelchair_accessible) WHERE wheelchair_accessible = true;
CREATE INDEX IF NOT EXISTS idx_service_listings_evening ON service_listings(evening_hours) WHERE evening_hours = true;

-- Step 5: Update table comment
COMMENT ON TABLE service_listings IS 'Generic service properties applicable across all service types (legal, healthcare, social services, etc.)';
COMMENT ON COLUMN service_listings.requires_identification IS 'Service requires government-issued ID';
COMMENT ON COLUMN service_listings.requires_appointment IS 'Must schedule appointment in advance';
COMMENT ON COLUMN service_listings.walk_ins_accepted IS 'Accepts walk-in clients without appointment';
COMMENT ON COLUMN service_listings.remote_available IS 'Offers remote/virtual services (phone, video, online)';
COMMENT ON COLUMN service_listings.in_person_available IS 'Offers in-person services at physical location';
COMMENT ON COLUMN service_listings.home_visits_available IS 'Provider travels to client location';
COMMENT ON COLUMN service_listings.wheelchair_accessible IS 'Physical location is wheelchair accessible';
COMMENT ON COLUMN service_listings.interpretation_available IS 'Provides language interpretation services';
COMMENT ON COLUMN service_listings.free_service IS 'Service is completely free';
COMMENT ON COLUMN service_listings.sliding_scale_fees IS 'Fees vary based on income/ability to pay';
COMMENT ON COLUMN service_listings.accepts_insurance IS 'Accepts health/other insurance';
COMMENT ON COLUMN service_listings.evening_hours IS 'Available during evening hours (after 5pm)';
COMMENT ON COLUMN service_listings.weekend_hours IS 'Available on weekends';

-- Step 6: Seed immigration safety tags
INSERT INTO tags (kind, value, display_name) VALUES
  ('safety', 'no_id_required', 'No ID Required'),
  ('safety', 'no_authority_contact', 'Does Not Contact Authorities'),
  ('safety', 'ice_safe', 'ICE Safe'),
  ('safety', 'community_based', 'Community-Based'),
  ('safety', 'confidential', 'Confidential Service'),
  ('safety', 'anonymous_ok', 'Anonymous Service Available'),
  ('safety', 'no_status_check', 'No Immigration Status Check'),
  ('safety', 'know_your_rights', 'Know Your Rights Info Provided')
ON CONFLICT (kind, value) DO NOTHING;

COMMENT ON TABLE tags IS 'Universal tags. Immigration safety tags (kind=safety) mark services safe for vulnerable populations.';

