-- Edition widgets: non-post content in broadsheet layouts (section headers, weather, hotline bars)
-- JSONB is appropriate here — each widget type has a genuinely different config shape.

CREATE TABLE edition_widgets (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    edition_row_id  UUID NOT NULL REFERENCES edition_rows(id) ON DELETE CASCADE,
    widget_type     TEXT NOT NULL,  -- 'section_header', 'weather', 'hotline_bar'
    slot_index      INT NOT NULL DEFAULT 0,
    config          JSONB NOT NULL DEFAULT '{}',
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_edition_widgets_row ON edition_widgets(edition_row_id);
