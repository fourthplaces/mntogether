-- Create locations as first-class entity
-- Aligns with HSDS 'location' and findhelp.org 'Location' entities
-- Enables multi-site services without post duplication

CREATE TABLE locations (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID REFERENCES organizations(id) ON DELETE SET NULL,
    name TEXT,
    address_line_1 TEXT,
    address_line_2 TEXT,
    city TEXT,
    state TEXT,
    postal_code TEXT,
    latitude DOUBLE PRECISION,
    longitude DOUBLE PRECISION,
    location_type TEXT NOT NULL DEFAULT 'physical', -- 'physical', 'virtual', 'postal'
    accessibility_notes TEXT,
    transportation_notes TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_locations_organization ON locations(organization_id);
CREATE INDEX idx_locations_type ON locations(location_type);
CREATE INDEX idx_locations_city_state ON locations(city, state);
CREATE INDEX idx_locations_postal ON locations(postal_code) WHERE postal_code IS NOT NULL;

COMMENT ON TABLE locations IS 'Physical, virtual, or postal locations where services are delivered (HSDS-aligned)';
COMMENT ON COLUMN locations.location_type IS 'physical, virtual, or postal (HSDS location_type)';
