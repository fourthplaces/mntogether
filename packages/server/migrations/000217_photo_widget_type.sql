-- Add 'photo' widget type for full-width editorial photo breaks.
-- These act as visual pacing between dense content sections,
-- matching the FeaturePhoto component from the design prototype.

ALTER TABLE widgets DROP CONSTRAINT IF EXISTS widgets_type_check;
ALTER TABLE widgets ADD CONSTRAINT widgets_type_check
  CHECK (widget_type = ANY (ARRAY['number', 'pull_quote', 'resource_bar', 'weather', 'section_sep', 'photo']));

-- Widget-trio and widget-pair row templates for multi-widget rows.
-- Stat cards render 3-up (trio), number blocks render 2-up (pair).
INSERT INTO row_template_configs (slug, display_name, layout_variant, sort_order)
VALUES ('widget-trio', 'Widget Trio (3-up)', 'trio', 99)
ON CONFLICT DO NOTHING;
INSERT INTO row_template_configs (slug, display_name, layout_variant, sort_order)
VALUES ('widget-pair', 'Widget Pair (2-up)', 'pair', 99)
ON CONFLICT DO NOTHING;
