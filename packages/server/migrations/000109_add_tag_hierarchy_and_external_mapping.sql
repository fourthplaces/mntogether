-- Add hierarchy and external taxonomy mapping to tags
-- Enables: parent-child tag browsing, crosswalking to Open Eligibility / 211HSIS

-- Self-referential FK for tag hierarchy (e.g., "Food" > "Food Pantries")
ALTER TABLE tags ADD COLUMN parent_tag_id UUID REFERENCES tags(id) ON DELETE SET NULL;

-- External taxonomy code (e.g., 'BD-1800.2000' for 211HSIS, '1102' for Open Eligibility)
ALTER TABLE tags ADD COLUMN external_code TEXT;

-- Which taxonomy system this tag maps to ('custom', 'open_eligibility', '211hsis')
ALTER TABLE tags ADD COLUMN taxonomy_source TEXT DEFAULT 'custom';

-- Index for hierarchy queries (find children of a parent)
CREATE INDEX idx_tags_parent ON tags(parent_tag_id) WHERE parent_tag_id IS NOT NULL;

-- Index for external code lookups
CREATE INDEX idx_tags_external ON tags(taxonomy_source, external_code) WHERE external_code IS NOT NULL;

COMMENT ON COLUMN tags.parent_tag_id IS 'Self-referential FK for tag hierarchy (e.g., Food > Food Pantries)';
COMMENT ON COLUMN tags.external_code IS 'Code in external taxonomy (e.g., BD-1800.2000 for 211HSIS)';
COMMENT ON COLUMN tags.taxonomy_source IS 'Taxonomy system: custom, open_eligibility, 211hsis';
