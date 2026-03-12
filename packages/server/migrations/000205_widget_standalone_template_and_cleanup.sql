-- =============================================================================
-- Widget-standalone row template + drop legacy edition_widgets
-- =============================================================================
-- 1. Seed a row template for standalone widgets (ResourceBar, SectionSep, etc.)
--    that render full-width without a Row/Cell wrapper.
-- 2. Drop the edition_widgets table (replaced by widgets + edition_slots).
-- =============================================================================

-- Standalone widget row template: single span-6 slot, renderer skips row wrapper
INSERT INTO row_template_configs (slug, display_name, description, sort_order, layout_variant) VALUES
('widget-standalone', 'Widget (Standalone)', 'Full-width widget without row wrapper', 99, 'widget-standalone')
ON CONFLICT (slug) DO NOTHING;

-- No row_template_slots needed — widget-standalone rows hold a single widget via edition_slots

-- Drop legacy edition_widgets table (all test data, no production records)
DROP TABLE IF EXISTS edition_widgets CASCADE;
