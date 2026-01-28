-- Add location coordinates to organization_needs for proximity matching
-- These are inherited from organization/source when need is created

ALTER TABLE organization_needs
    ADD COLUMN latitude NUMERIC(9, 6),
    ADD COLUMN longitude NUMERIC(9, 6);

-- Add constraints
ALTER TABLE organization_needs ADD CONSTRAINT chk_needs_lat CHECK (latitude BETWEEN -90 AND 90);
ALTER TABLE organization_needs ADD CONSTRAINT chk_needs_lng CHECK (longitude BETWEEN -180 AND 180);

-- Indexes for spatial queries (used in matching algorithm)
CREATE INDEX idx_needs_lat ON organization_needs(latitude) WHERE latitude IS NOT NULL;
CREATE INDEX idx_needs_lng ON organization_needs(longitude) WHERE longitude IS NOT NULL;

COMMENT ON COLUMN organization_needs.latitude IS 'Latitude inherited from organization/source for proximity matching';
COMMENT ON COLUMN organization_needs.longitude IS 'Longitude inherited from organization/source for proximity matching';
