-- Universal tags table (not org-specific)
-- Can be used for organizations, needs, volunteers, etc.

CREATE TABLE tags (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    -- Tag classification
    kind TEXT NOT NULL,  -- 'service', 'language', 'community', etc.
    value TEXT NOT NULL, -- 'food_assistance', 'spanish', 'somali', etc.

    -- Prevent duplicate tag definitions
    UNIQUE(kind, value),

    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Index for finding tags by kind
CREATE INDEX idx_tags_kind ON tags(kind);

-- Index for finding specific tag
CREATE INDEX idx_tags_kind_value ON tags(kind, value);

COMMENT ON TABLE tags IS 'Universal tag definitions. Can be associated with any entity via junction tables.';
COMMENT ON COLUMN tags.kind IS 'Tag category: service, language, community, urgency, etc.';
COMMENT ON COLUMN tags.value IS 'Tag value: e.g., food_assistance, spanish, somali, urgent';

-- Insert common tags
INSERT INTO tags (kind, value) VALUES
    -- Service types
    ('service', 'food_assistance'),
    ('service', 'housing_assistance'),
    ('service', 'legal_services'),
    ('service', 'employment_support'),
    ('service', 'emergency_financial_aid'),
    ('service', 'shelter'),
    ('service', 'utility_assistance'),

    -- Languages
    ('language', 'english'),
    ('language', 'spanish'),
    ('language', 'somali'),
    ('language', 'hmong'),
    ('language', 'karen'),
    ('language', 'vietnamese'),
    ('language', 'arabic'),

    -- Communities
    ('community', 'general'),
    ('community', 'latino'),
    ('community', 'somali'),
    ('community', 'hmong'),
    ('community', 'karen'),
    ('community', 'vietnamese'),
    ('community', 'east_african'),
    ('community', 'native_american')
ON CONFLICT (kind, value) DO NOTHING;
