-- =============================================================================
-- Widgets become edition-level layout items (peers of rows, not children)
-- =============================================================================
-- Previously widgets were children of rows (edition_row_id FK).
-- Now they are independent items with their own sort_order, sitting alongside
-- rows in the layout. They can optionally belong to a section.

-- Add new columns
ALTER TABLE edition_widgets ADD COLUMN edition_id UUID REFERENCES editions(id) ON DELETE CASCADE;
ALTER TABLE edition_widgets ADD COLUMN sort_order INT NOT NULL DEFAULT 0;
ALTER TABLE edition_widgets ADD COLUMN section_id UUID REFERENCES edition_sections(id) ON DELETE SET NULL;

-- Make edition_row_id nullable (legacy, will be unused going forward)
ALTER TABLE edition_widgets ALTER COLUMN edition_row_id DROP NOT NULL;

-- Backfill: copy edition_id and section_id from the parent row,
-- place widget just before its row in sort order
UPDATE edition_widgets w
SET edition_id = r.edition_id,
    sort_order = r.sort_order * 10 - 1,
    section_id = r.section_id
FROM edition_rows r
WHERE w.edition_row_id = r.id;

-- Now enforce edition_id NOT NULL
ALTER TABLE edition_widgets ALTER COLUMN edition_id SET NOT NULL;

-- Index for querying widgets by edition
CREATE INDEX idx_edition_widgets_edition ON edition_widgets(edition_id);
