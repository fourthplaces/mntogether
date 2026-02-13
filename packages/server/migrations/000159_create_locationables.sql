-- Polymorphic join table linking locations to any entity (mirrors noteables pattern).
-- Enables any entity (post, organization, etc.) to have locations.
CREATE TABLE locationables (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    location_id UUID NOT NULL REFERENCES locations(id) ON DELETE CASCADE,
    locatable_type TEXT NOT NULL,
    locatable_id UUID NOT NULL,
    is_primary BOOLEAN NOT NULL DEFAULT false,
    notes TEXT,
    added_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(location_id, locatable_type, locatable_id)
);

CREATE INDEX idx_locationables_entity ON locationables(locatable_type, locatable_id);
CREATE INDEX idx_locationables_location ON locationables(location_id);
