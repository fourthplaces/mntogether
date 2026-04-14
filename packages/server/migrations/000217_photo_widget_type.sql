-- Add 'photo' widget type for full-width editorial photo breaks.
-- These act as visual pacing between dense content sections,
-- matching the FeaturePhoto component from the design prototype.

ALTER TABLE widgets DROP CONSTRAINT IF EXISTS widgets_type_check;
ALTER TABLE widgets ADD CONSTRAINT widgets_type_check
  CHECK (widget_type = ANY (ARRAY['number', 'pull_quote', 'resource_bar', 'weather', 'section_sep', 'photo']));
