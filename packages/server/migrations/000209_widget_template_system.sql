-- =============================================================================
-- Widget Template System
-- =============================================================================
-- 1. Add widget_template column to edition_slots (parallel to post_template)
-- 2. Merge stat_card + number_block → number type with variants
--
-- See DECISIONS_LOG.md: "Widget template system: Merge stat_card + number_block"
-- =============================================================================

-- 1. Add widget_template to edition_slots
ALTER TABLE edition_slots ADD COLUMN IF NOT EXISTS widget_template TEXT;

-- 2. Migrate stat_card → number (variant stored in widget_template on slot)
UPDATE widgets SET widget_type = 'number' WHERE widget_type = 'stat_card';
UPDATE widgets SET widget_type = 'number' WHERE widget_type = 'number_block';

-- Set widget_template on existing number widget slots based on original type
-- (We can't know the original type after rename, but existing slots can default)
UPDATE edition_slots SET widget_template = 'stat-card'
WHERE widget_id IN (SELECT id FROM widgets WHERE widget_type = 'number')
  AND widget_template IS NULL;
