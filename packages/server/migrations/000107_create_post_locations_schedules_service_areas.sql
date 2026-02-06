-- Create structural tables: post_locations, schedules, service_areas, taxonomy_crosswalks
-- These enable multi-site services, operating hours, geographic coverage, and taxonomy interop

-- =============================================================================
-- post_locations: Many-to-many join between posts and locations
-- Aligns with HSDS 'service_at_location'
-- =============================================================================

CREATE TABLE post_locations (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    post_id UUID NOT NULL REFERENCES posts(id) ON DELETE CASCADE,
    location_id UUID NOT NULL REFERENCES locations(id) ON DELETE CASCADE,
    is_primary BOOLEAN NOT NULL DEFAULT false,
    notes TEXT, -- location-specific service notes
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(post_id, location_id)
);

CREATE INDEX idx_post_locations_post ON post_locations(post_id);
CREATE INDEX idx_post_locations_location ON post_locations(location_id);

COMMENT ON TABLE post_locations IS 'Links posts to locations (HSDS service_at_location equivalent)';

-- =============================================================================
-- schedules: Polymorphic operating hours
-- Aligns with HSDS 'schedule' and findhelp per-location hours
-- =============================================================================

CREATE TABLE schedules (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    schedulable_type TEXT NOT NULL, -- 'post', 'location', 'post_location'
    schedulable_id UUID NOT NULL,
    day_of_week INTEGER NOT NULL CHECK (day_of_week BETWEEN 0 AND 6), -- 0=Sunday, 6=Saturday
    opens_at TIME,
    closes_at TIME,
    timezone TEXT NOT NULL DEFAULT 'America/Chicago',
    valid_from DATE,
    valid_to DATE,
    notes TEXT, -- 'By appointment only', 'Walk-ins welcome', 'Closed for holidays'
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_schedules_schedulable ON schedules(schedulable_type, schedulable_id);
CREATE INDEX idx_schedules_day ON schedules(day_of_week);

COMMENT ON TABLE schedules IS 'Operating hours for posts, locations, or post_locations (polymorphic)';
COMMENT ON COLUMN schedules.schedulable_type IS 'post, location, or post_location';
COMMENT ON COLUMN schedules.day_of_week IS '0=Sunday through 6=Saturday';

-- =============================================================================
-- service_areas: Geographic coverage for posts
-- Aligns with HSDS 'service_area'
-- =============================================================================

CREATE TABLE service_areas (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    post_id UUID NOT NULL REFERENCES posts(id) ON DELETE CASCADE,
    area_type TEXT NOT NULL, -- 'county', 'city', 'state', 'zip', 'custom'
    area_name TEXT NOT NULL, -- 'Hennepin County', 'Minneapolis', 'MN'
    area_code TEXT, -- FIPS code, ZIP code, state abbreviation, etc.
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_service_areas_post ON service_areas(post_id);
CREATE INDEX idx_service_areas_type_name ON service_areas(area_type, area_name);
CREATE INDEX idx_service_areas_code ON service_areas(area_code) WHERE area_code IS NOT NULL;

COMMENT ON TABLE service_areas IS 'Geographic coverage areas for posts (HSDS service_area equivalent)';
COMMENT ON COLUMN service_areas.area_type IS 'county, city, state, zip, or custom';

-- =============================================================================
-- taxonomy_crosswalks: Map internal tags to external taxonomy systems
-- Enables interop with 211, findhelp, NTEE, etc.
-- =============================================================================

CREATE TABLE taxonomy_crosswalks (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tag_id UUID NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
    external_system TEXT NOT NULL, -- 'open_eligibility', '211hsis', 'ntee'
    external_code TEXT NOT NULL, -- 'BD-1800.2000', '1102', etc.
    external_name TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(tag_id, external_system)
);

CREATE INDEX idx_crosswalks_tag ON taxonomy_crosswalks(tag_id);
CREATE INDEX idx_crosswalks_external ON taxonomy_crosswalks(external_system, external_code);

COMMENT ON TABLE taxonomy_crosswalks IS 'Maps internal tags to external taxonomy codes (211HSIS, Open Eligibility, NTEE)';
