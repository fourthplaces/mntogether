-- Add location coordinates for distance-based matching
-- Uses simple lat/lng columns (no PostGIS needed for MVP)

-- 1. Rename volunteers → members
ALTER TABLE volunteers RENAME TO members;
ALTER INDEX idx_volunteers_token RENAME TO idx_members_token;
ALTER INDEX idx_volunteers_active RENAME TO idx_members_active;

-- 2. Clean up members table - drop old geolocation columns
ALTER TABLE members
    DROP COLUMN IF EXISTS availability,
    DROP COLUMN IF EXISTS location,
    DROP COLUMN IF EXISTS ip_address,
    DROP COLUMN IF EXISTS city,
    DROP COLUMN IF EXISTS state,
    DROP COLUMN IF EXISTS country;

-- 3. Update lat/lng precision and add location_name
ALTER TABLE members
    DROP COLUMN IF EXISTS latitude,
    DROP COLUMN IF EXISTS longitude;

ALTER TABLE members
    ADD COLUMN latitude NUMERIC(9, 6),    -- 6 decimal places ≈ 0.1m precision
    ADD COLUMN longitude NUMERIC(9, 6),
    ADD COLUMN location_name TEXT;        -- "Minneapolis, MN" for display

-- Add constraints
ALTER TABLE members ADD CONSTRAINT chk_members_lat CHECK (latitude BETWEEN -90 AND 90);
ALTER TABLE members ADD CONSTRAINT chk_members_lng CHECK (longitude BETWEEN -180 AND 180);

-- Indexes for spatial queries
CREATE INDEX idx_members_lat ON members(latitude) WHERE latitude IS NOT NULL;
CREATE INDEX idx_members_lng ON members(longitude) WHERE longitude IS NOT NULL;

-- 4. Add location to organizations
ALTER TABLE organizations
    ADD COLUMN latitude NUMERIC(9, 6),
    ADD COLUMN longitude NUMERIC(9, 6),
    ADD COLUMN location_name TEXT;  -- Override/display name

ALTER TABLE organizations ADD CONSTRAINT chk_orgs_lat CHECK (latitude BETWEEN -90 AND 90);
ALTER TABLE organizations ADD CONSTRAINT chk_orgs_lng CHECK (longitude BETWEEN -180 AND 180);

CREATE INDEX idx_orgs_lat ON organizations(latitude) WHERE latitude IS NOT NULL;
CREATE INDEX idx_orgs_lng ON organizations(longitude) WHERE longitude IS NOT NULL;

-- 5. Add location to organization_sources (for context during scraping)
ALTER TABLE organization_sources
    ADD COLUMN latitude NUMERIC(9, 6),
    ADD COLUMN longitude NUMERIC(9, 6),
    ADD COLUMN location_name TEXT;

ALTER TABLE organization_sources ADD CONSTRAINT chk_sources_lat CHECK (latitude BETWEEN -90 AND 90);
ALTER TABLE organization_sources ADD CONSTRAINT chk_sources_lng CHECK (longitude BETWEEN -180 AND 180);

-- 6. Add Haversine distance calculation function
CREATE OR REPLACE FUNCTION haversine_distance(
    lat1 NUMERIC, lng1 NUMERIC,
    lat2 NUMERIC, lng2 NUMERIC
) RETURNS NUMERIC AS $$
DECLARE
    r NUMERIC := 6371; -- Earth radius in kilometers
    dlat NUMERIC;
    dlng NUMERIC;
    a NUMERIC;
    c NUMERIC;
BEGIN
    -- Handle NULL inputs
    IF lat1 IS NULL OR lng1 IS NULL OR lat2 IS NULL OR lng2 IS NULL THEN
        RETURN NULL;
    END IF;

    dlat := radians(lat2 - lat1);
    dlng := radians(lng2 - lng1);

    a := sin(dlat/2) * sin(dlat/2) +
         cos(radians(lat1)) * cos(radians(lat2)) *
         sin(dlng/2) * sin(dlng/2);

    c := 2 * atan2(sqrt(a), sqrt(1-a));

    RETURN r * c; -- Distance in kilometers
END;
$$ LANGUAGE plpgsql IMMUTABLE;

-- 7. Update comments
COMMENT ON TABLE members IS 'Privacy-first member registry (zero PII, only expo_push_token)';
COMMENT ON COLUMN members.searchable_text IS 'TEXT-FIRST source of truth: all capabilities, skills, interests';
COMMENT ON COLUMN members.latitude IS 'Coarse latitude (city-level precision, 2 decimal places stored) - required for matching';
COMMENT ON COLUMN members.longitude IS 'Coarse longitude (city-level precision, 2 decimal places stored) - required for matching';
COMMENT ON COLUMN members.location_name IS 'Human-readable location for display (e.g., "Minneapolis, MN")';

COMMENT ON COLUMN organizations.latitude IS 'Organization latitude (city-level) - geocoded from city/state or manually set';
COMMENT ON COLUMN organizations.longitude IS 'Organization longitude (city-level) - geocoded from city/state or manually set';
COMMENT ON COLUMN organizations.location_name IS 'Override location name for display (defaults to city, state)';

COMMENT ON COLUMN organization_sources.latitude IS 'Source location for contextual scraping';
COMMENT ON COLUMN organization_sources.longitude IS 'Source location for contextual scraping';
COMMENT ON COLUMN organization_sources.location_name IS 'Location context for AI extraction';

COMMENT ON FUNCTION haversine_distance IS 'Calculate distance in kilometers between two lat/lng points using Haversine formula';
