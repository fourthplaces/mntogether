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

-- 2. Drop old CHECK constraint before renaming types
ALTER TABLE widgets DROP CONSTRAINT IF EXISTS widgets_type_check;

-- 3. Migrate stat_card/number_block → number
UPDATE widgets SET widget_type = 'number' WHERE widget_type = 'stat_card';
UPDATE widgets SET widget_type = 'number' WHERE widget_type = 'number_block';

-- 4. Add new CHECK constraint with merged 'number' type
ALTER TABLE widgets ADD CONSTRAINT widgets_type_check CHECK (
    widget_type IN ('number', 'pull_quote', 'resource_bar', 'weather', 'section_sep')
);

-- 5. Set widget_template on existing number widget slots
UPDATE edition_slots SET widget_template = 'stat-card'
WHERE widget_id IN (SELECT id FROM widgets WHERE widget_type = 'number')
  AND widget_template IS NULL;
