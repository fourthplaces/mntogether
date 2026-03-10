-- =============================================================================
-- Row templates as curated recipes
-- =============================================================================
-- Each row template now specifies:
--   layout_variant: CSS grid layout (many templates can share a layout)
--   post_template_slug on each slot: the "good default" post template
--
-- Row templates are designer-curated pairings. The layout engine uses them to
-- auto-assign posts. Editors can override individual post templates per slot
-- without changing the underlying recipe.
-- =============================================================================

-- 1. Add layout_variant to row_template_configs
ALTER TABLE row_template_configs ADD COLUMN layout_variant TEXT NOT NULL DEFAULT 'full';

-- 2. Add post_template_slug to row_template_slots
ALTER TABLE row_template_slots ADD COLUMN post_template_slug TEXT;

-- =============================================================================
-- Backfill layout_variant for existing templates
-- =============================================================================
UPDATE row_template_configs SET layout_variant = 'lead-stack' WHERE slug = 'hero-with-sidebar';
UPDATE row_template_configs SET layout_variant = 'full'       WHERE slug IN ('hero-full', 'ticker', 'single-medium');
UPDATE row_template_configs SET layout_variant = 'trio'       WHERE slug IN ('three-column', 'classifieds');
UPDATE row_template_configs SET layout_variant = 'lead'       WHERE slug = 'two-column-wide-narrow';
UPDATE row_template_configs SET layout_variant = 'pair'       WHERE slug = 'four-column';

-- =============================================================================
-- Backfill post_template_slug for existing template slots
-- =============================================================================

-- hero-with-sidebar: feature + 3×digest
UPDATE row_template_slots SET post_template_slug = 'feature'
  WHERE row_template_config_id = (SELECT id FROM row_template_configs WHERE slug = 'hero-with-sidebar')
  AND slot_index = 0;
UPDATE row_template_slots SET post_template_slug = 'digest'
  WHERE row_template_config_id = (SELECT id FROM row_template_configs WHERE slug = 'hero-with-sidebar')
  AND slot_index = 1;

-- hero-full: feature
UPDATE row_template_slots SET post_template_slug = 'feature'
  WHERE row_template_config_id = (SELECT id FROM row_template_configs WHERE slug = 'hero-full')
  AND slot_index = 0;

-- three-column: 3×gazette
UPDATE row_template_slots SET post_template_slug = 'gazette'
  WHERE row_template_config_id = (SELECT id FROM row_template_configs WHERE slug = 'three-column')
  AND slot_index = 0;

-- two-column-wide-narrow: feature + bulletin
UPDATE row_template_slots SET post_template_slug = 'feature'
  WHERE row_template_config_id = (SELECT id FROM row_template_configs WHERE slug = 'two-column-wide-narrow')
  AND slot_index = 0;
UPDATE row_template_slots SET post_template_slug = 'bulletin'
  WHERE row_template_config_id = (SELECT id FROM row_template_configs WHERE slug = 'two-column-wide-narrow')
  AND slot_index = 1;

-- four-column: 4×gazette
UPDATE row_template_slots SET post_template_slug = 'gazette'
  WHERE row_template_config_id = (SELECT id FROM row_template_configs WHERE slug = 'four-column')
  AND slot_index = 0;

-- classifieds: 6×digest
UPDATE row_template_slots SET post_template_slug = 'digest'
  WHERE row_template_config_id = (SELECT id FROM row_template_configs WHERE slug = 'classifieds')
  AND slot_index = 0;

-- ticker: 8×ticker
UPDATE row_template_slots SET post_template_slug = 'ticker'
  WHERE row_template_config_id = (SELECT id FROM row_template_configs WHERE slug = 'ticker')
  AND slot_index = 0;

-- single-medium: gazette
UPDATE row_template_slots SET post_template_slug = 'gazette'
  WHERE row_template_config_id = (SELECT id FROM row_template_configs WHERE slug = 'single-medium')
  AND slot_index = 0;

-- =============================================================================
-- New curated row template recipes
-- These reuse existing layout_variants with different post template pairings.
-- =============================================================================

-- Lead-stack recipes (hero layout: 1 heavy + 3 light)
INSERT INTO row_template_configs (slug, display_name, description, sort_order, layout_variant) VALUES
('hero-feature-ticker',  'Feature + Ticker sidebar',  'Feature hero with ticker items in sidebar',  10, 'lead-stack'),
('hero-feature-ledger',  'Feature + Ledger sidebar',  'Feature hero with ledger items in sidebar',  11, 'lead-stack')
ON CONFLICT (slug) DO NOTHING;

INSERT INTO row_template_slots (row_template_config_id, slot_index, weight, count, post_template_slug) VALUES
((SELECT id FROM row_template_configs WHERE slug = 'hero-feature-ticker'), 0, 'heavy', 1, 'feature'),
((SELECT id FROM row_template_configs WHERE slug = 'hero-feature-ticker'), 1, 'light', 3, 'ticker'),
((SELECT id FROM row_template_configs WHERE slug = 'hero-feature-ledger'), 0, 'heavy', 1, 'feature'),
((SELECT id FROM row_template_configs WHERE slug = 'hero-feature-ledger'), 1, 'light', 3, 'ledger');

-- Trio recipes (3-column layout: 3 medium)
INSERT INTO row_template_configs (slug, display_name, description, sort_order, layout_variant) VALUES
('trio-gazette-bulletin',  'Gazette + Bulletin + Gazette',   'Mixed gazette/bulletin trio',              12, 'trio'),
('trio-bulletin',          'Triple Bulletin',                'Three bulletin cards',                     13, 'trio'),
('trio-mixed-spotlight',   'Gazette + Spotlight + Gazette',  'Spotlight center with gazette flanks',     14, 'trio')
ON CONFLICT (slug) DO NOTHING;

INSERT INTO row_template_slots (row_template_config_id, slot_index, weight, count, post_template_slug) VALUES
-- trio-gazette-bulletin: gazette, bulletin, gazette (1 post each)
((SELECT id FROM row_template_configs WHERE slug = 'trio-gazette-bulletin'), 0, 'medium', 1, 'gazette'),
((SELECT id FROM row_template_configs WHERE slug = 'trio-gazette-bulletin'), 1, 'medium', 1, 'bulletin'),
((SELECT id FROM row_template_configs WHERE slug = 'trio-gazette-bulletin'), 2, 'medium', 1, 'gazette'),
-- trio-bulletin
((SELECT id FROM row_template_configs WHERE slug = 'trio-bulletin'), 0, 'medium', 1, 'bulletin'),
((SELECT id FROM row_template_configs WHERE slug = 'trio-bulletin'), 1, 'medium', 1, 'bulletin'),
((SELECT id FROM row_template_configs WHERE slug = 'trio-bulletin'), 2, 'medium', 1, 'bulletin'),
-- trio-mixed-spotlight
((SELECT id FROM row_template_configs WHERE slug = 'trio-mixed-spotlight'), 0, 'medium', 1, 'gazette'),
((SELECT id FROM row_template_configs WHERE slug = 'trio-mixed-spotlight'), 1, 'medium', 1, 'spotlight-local'),
((SELECT id FROM row_template_configs WHERE slug = 'trio-mixed-spotlight'), 2, 'medium', 1, 'gazette');

-- Classifieds recipes (3-column layout: 6 light, 2 per column)
INSERT INTO row_template_configs (slug, display_name, description, sort_order, layout_variant) VALUES
('classifieds-ledger',    'Classifieds (Ledger)',      'Compact ledger listings in 3 columns',     15, 'trio'),
('classifieds-ticker',    'Classifieds (Ticker)',      'Ticker-style compact listings',            16, 'trio')
ON CONFLICT (slug) DO NOTHING;

INSERT INTO row_template_slots (row_template_config_id, slot_index, weight, count, post_template_slug) VALUES
((SELECT id FROM row_template_configs WHERE slug = 'classifieds-ledger'), 0, 'light', 2, 'ledger'),
((SELECT id FROM row_template_configs WHERE slug = 'classifieds-ledger'), 1, 'light', 2, 'ledger'),
((SELECT id FROM row_template_configs WHERE slug = 'classifieds-ledger'), 2, 'light', 2, 'ledger'),
((SELECT id FROM row_template_configs WHERE slug = 'classifieds-ticker'), 0, 'light', 2, 'ticker'),
((SELECT id FROM row_template_configs WHERE slug = 'classifieds-ticker'), 1, 'light', 2, 'ticker'),
((SELECT id FROM row_template_configs WHERE slug = 'classifieds-ticker'), 2, 'light', 2, 'ticker');

-- Lead recipes (wide + narrow: 1 heavy + 1 medium)
INSERT INTO row_template_configs (slug, display_name, description, sort_order, layout_variant) VALUES
('lead-feature-gazette',    'Feature + Gazette',    'Feature story with gazette sidebar',    17, 'lead'),
('lead-reversed-bulletin',  'Alert + Bulletin',     'Reversed feature with bulletin aside',  18, 'lead')
ON CONFLICT (slug) DO NOTHING;

INSERT INTO row_template_slots (row_template_config_id, slot_index, weight, count, post_template_slug) VALUES
((SELECT id FROM row_template_configs WHERE slug = 'lead-feature-gazette'), 0, 'heavy', 1, 'feature'),
((SELECT id FROM row_template_configs WHERE slug = 'lead-feature-gazette'), 1, 'medium', 1, 'gazette'),
((SELECT id FROM row_template_configs WHERE slug = 'lead-reversed-bulletin'), 0, 'heavy', 1, 'feature-reversed'),
((SELECT id FROM row_template_configs WHERE slug = 'lead-reversed-bulletin'), 1, 'medium', 1, 'bulletin');

-- Full-width single recipes
INSERT INTO row_template_configs (slug, display_name, description, sort_order, layout_variant) VALUES
('single-bulletin',    'Single Bulletin',      'Standalone bulletin card',     19, 'full'),
('single-feature',     'Single Feature',       'Standalone feature (hero)',    20, 'full')
ON CONFLICT (slug) DO NOTHING;

INSERT INTO row_template_slots (row_template_config_id, slot_index, weight, count, post_template_slug) VALUES
((SELECT id FROM row_template_configs WHERE slug = 'single-bulletin'), 0, 'medium', 1, 'bulletin'),
((SELECT id FROM row_template_configs WHERE slug = 'single-feature'), 0, 'heavy', 1, 'feature');

-- Ticker update recipe (light notices as ticker-updates)
INSERT INTO row_template_configs (slug, display_name, description, sort_order, layout_variant) VALUES
('ticker-updates', 'Ticker Updates', 'Ticker strip of notice updates', 21, 'full')
ON CONFLICT (slug) DO NOTHING;

INSERT INTO row_template_slots (row_template_config_id, slot_index, weight, count, post_template_slug) VALUES
((SELECT id FROM row_template_configs WHERE slug = 'ticker-updates'), 0, 'light', 8, 'ticker-update');
