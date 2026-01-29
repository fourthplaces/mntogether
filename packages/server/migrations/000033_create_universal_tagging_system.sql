-- Create universal tagging system (flexible metadata for all entities)

-- Step 1: Drop and recreate tags table with new schema
DROP TABLE IF EXISTS tags CASCADE;

CREATE TABLE tags (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  kind TEXT NOT NULL,
  value TEXT NOT NULL,
  display_name TEXT,
  created_at TIMESTAMPTZ DEFAULT NOW(),
  UNIQUE(kind, value)
);

CREATE INDEX idx_tags_kind ON tags(kind);
CREATE INDEX idx_tags_value ON tags(value);

COMMENT ON TABLE tags IS 'Universal tags for flexible metadata (community_served, service_area, population, etc.)';
COMMENT ON COLUMN tags.kind IS 'Tag type (community_served, service_area, population, org_leadership, etc.)';
COMMENT ON COLUMN tags.value IS 'Tag value (somali, minneapolis, seniors, etc.)';
COMMENT ON COLUMN tags.display_name IS 'Human-readable name for UI';

-- Step 2: Create polymorphic tagging table
CREATE TABLE taggables (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  tag_id UUID NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
  taggable_type TEXT NOT NULL,
  taggable_id UUID NOT NULL,
  added_at TIMESTAMPTZ DEFAULT NOW(),
  UNIQUE(tag_id, taggable_type, taggable_id)
);

CREATE INDEX idx_taggables_tag ON taggables(tag_id);
CREATE INDEX idx_taggables_entity ON taggables(taggable_type, taggable_id);
CREATE INDEX idx_taggables_type ON taggables(taggable_type);

COMMENT ON TABLE taggables IS 'Polymorphic join table: links tags to any entity (listing, organization, document, domain)';
COMMENT ON COLUMN taggables.taggable_type IS 'Entity type: listing, organization, referral_document, domain';
COMMENT ON COLUMN taggables.taggable_id IS 'UUID of the tagged entity';

-- Step 3: Seed MVP tags

-- Community served tags
INSERT INTO tags (kind, value, display_name) VALUES
  ('community_served', 'somali', 'Somali'),
  ('community_served', 'ethiopian', 'Ethiopian'),
  ('community_served', 'latino', 'Latino'),
  ('community_served', 'hmong', 'Hmong'),
  ('community_served', 'karen', 'Karen'),
  ('community_served', 'oromo', 'Oromo');

-- Service area tags
INSERT INTO tags (kind, value, display_name) VALUES
  ('service_area', 'minneapolis', 'Minneapolis'),
  ('service_area', 'st_paul', 'St. Paul'),
  ('service_area', 'bloomington', 'Bloomington'),
  ('service_area', 'brooklyn_park', 'Brooklyn Park'),
  ('service_area', 'statewide', 'Statewide');

-- Population served tags
INSERT INTO tags (kind, value, display_name) VALUES
  ('population', 'seniors', 'Seniors'),
  ('population', 'youth', 'Youth'),
  ('population', 'families', 'Families with Children'),
  ('population', 'veterans', 'Veterans'),
  ('population', 'lgbtq', 'LGBTQ+');

-- Organization metadata tags
INSERT INTO tags (kind, value, display_name) VALUES
  ('org_leadership', 'community_led', 'Community-Led'),
  ('org_leadership', 'immigrant_founded', 'Immigrant-Founded'),
  ('org_leadership', 'bipoc_led', 'BIPOC-Led');

-- Verification source tags (richer than boolean)
INSERT INTO tags (kind, value, display_name) VALUES
  ('verification_source', 'admin_verified', 'Admin Verified'),
  ('verification_source', 'community_vouched', 'Community Vouched'),
  ('verification_source', 'self_reported', 'Self-Reported');

COMMENT ON TABLE tags IS 'Hot path fields (category, capacity, urgency) are columns. Discovery metadata (community, service_area, population) are tags.';
