-- =============================================================================
-- Standalone widgets table
-- =============================================================================
-- Widgets are CMS-authored content items (like posts) with a discriminated
-- union on widget_type. Each type has its own data shape stored in JSONB,
-- validated at the application layer. Widgets slot into edition rows via
-- edition_slots, the same mechanism posts use.
-- =============================================================================

CREATE TABLE widgets (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    widget_type     TEXT NOT NULL,           -- 'stat_card', 'number_block', 'pull_quote', 'resource_bar', 'weather', 'section_sep'
    authoring_mode  TEXT NOT NULL DEFAULT 'human',  -- 'human', 'automated', 'layout'
    data            JSONB NOT NULL DEFAULT '{}',
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_widgets_type ON widgets(widget_type);

-- Validate widget_type is a known value
ALTER TABLE widgets ADD CONSTRAINT widgets_type_check CHECK (
    widget_type IN ('stat_card', 'number_block', 'pull_quote', 'resource_bar', 'weather', 'section_sep')
);

-- Validate authoring_mode
ALTER TABLE widgets ADD CONSTRAINT widgets_authoring_mode_check CHECK (
    authoring_mode IN ('human', 'automated', 'layout')
);
