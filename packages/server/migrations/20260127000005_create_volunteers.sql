-- Volunteers table (for SPIKE 2, but needed now for FK reference)

CREATE TABLE volunteers (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    -- Anonymous identifier (no PII)
    expo_push_token TEXT UNIQUE NOT NULL,

    -- TEXT-FIRST: Everything goes into searchable text
    searchable_text TEXT NOT NULL,

    -- Optional parsed hints
    availability TEXT,
    location TEXT,

    -- Geolocation (for matching)
    ip_address INET,
    city TEXT,
    state TEXT,
    country TEXT DEFAULT 'US',
    latitude NUMERIC(10, 8),
    longitude NUMERIC(11, 8),

    -- Status
    active BOOLEAN DEFAULT true,
    notification_count_this_week INTEGER DEFAULT 0,
    paused_until TIMESTAMPTZ,

    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_volunteers_token ON volunteers(expo_push_token);
CREATE INDEX idx_volunteers_active ON volunteers(active) WHERE active = true;

COMMENT ON TABLE volunteers IS 'Privacy-first volunteer registry (zero PII, only expo_push_token)';
COMMENT ON COLUMN volunteers.searchable_text IS 'TEXT-FIRST source of truth: all capabilities, skills, interests, availability combined';
COMMENT ON COLUMN volunteers.ip_address IS 'IP address for geolocation (approximate location only)';
COMMENT ON COLUMN volunteers.city IS 'Geolocated city from IP (e.g., "Minneapolis")';
COMMENT ON COLUMN volunteers.state IS 'Geolocated state from IP (e.g., "Minnesota")';
COMMENT ON COLUMN volunteers.latitude IS 'Approximate latitude from IP geolocation';
COMMENT ON COLUMN volunteers.longitude IS 'Approximate longitude from IP geolocation';
