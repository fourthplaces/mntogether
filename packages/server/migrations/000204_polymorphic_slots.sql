-- =============================================================================
-- Make edition_slots polymorphic (post OR widget)
-- =============================================================================
-- Adds a kind discriminator and widget_id FK. post_id and post_template
-- become nullable for widget slots. A CHECK constraint ensures exactly
-- one of post_id or widget_id is set depending on kind.
-- =============================================================================

-- Add kind discriminator (default 'post' so existing rows are valid)
ALTER TABLE edition_slots ADD COLUMN kind TEXT NOT NULL DEFAULT 'post';

-- Make post_id nullable (widget slots don't have a post)
ALTER TABLE edition_slots ALTER COLUMN post_id DROP NOT NULL;

-- Make post_template nullable (widget slots don't use post templates)
ALTER TABLE edition_slots ALTER COLUMN post_template DROP NOT NULL;

-- Add widget FK
ALTER TABLE edition_slots ADD COLUMN widget_id UUID REFERENCES widgets(id) ON DELETE CASCADE;

-- Ensure exactly one content reference per slot
ALTER TABLE edition_slots ADD CONSTRAINT edition_slots_kind_check CHECK (
    (kind = 'post' AND post_id IS NOT NULL AND widget_id IS NULL) OR
    (kind = 'widget' AND widget_id IS NOT NULL AND post_id IS NULL)
);

-- Index for widget lookups
CREATE INDEX idx_edition_slots_widget ON edition_slots(widget_id) WHERE widget_id IS NOT NULL;
