-- =============================================================================
-- Edition sections — topic-grouped row containers
-- =============================================================================
-- Sections allow grouping edition rows by topic (from Root Signal) or editorial
-- choice. Each section has a title, optional subtitle, and a topic_slug linking
-- to the Root Signal topic taxonomy.
--
-- edition_rows gain a nullable section_id FK. Rows with section_id=NULL render
-- "above the fold" (hero rows before any section divider). Deleting a section
-- sets rows' section_id to NULL (they become ungrouped).
-- =============================================================================

CREATE TABLE edition_sections (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    edition_id  UUID NOT NULL REFERENCES editions(id) ON DELETE CASCADE,
    title       TEXT NOT NULL,
    subtitle    TEXT,
    topic_slug  TEXT,
    sort_order  INT NOT NULL DEFAULT 0,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_edition_sections_edition ON edition_sections(edition_id);

-- Add section_id to edition_rows (nullable — null = above the fold)
ALTER TABLE edition_rows ADD COLUMN section_id UUID REFERENCES edition_sections(id) ON DELETE SET NULL;

CREATE INDEX idx_edition_rows_section ON edition_rows(section_id);
