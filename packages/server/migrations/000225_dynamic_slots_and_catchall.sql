-- ═══════════════════════════════════════════════════════════════════════
-- Dynamic slot counts + catchall row templates
--
-- Previously `row_template_slots.count` was exact — a row with count=3
-- required exactly 3 posts to fill, or the whole row was abandoned.
-- This caused good content to be dropped in small-county pools.
--
-- New semantics:
--   count_min — minimum posts to emit the row at all (default 1)
--   count_max — maximum posts the row will absorb (default = count)
--   count     — target count (used for height balancing and sorting)
--
-- The layout engine fills each slot up to count_max, accepts anything
-- at or above count_min, and only abandons the row if below count_min.
-- ═══════════════════════════════════════════════════════════════════════

ALTER TABLE row_template_slots
    ADD COLUMN count_min integer,
    ADD COLUMN count_max integer;

-- Backfill: existing behavior (exact count) = count_min=count_max=count
UPDATE row_template_slots SET count_min = count, count_max = count;

ALTER TABLE row_template_slots
    ALTER COLUMN count_min SET NOT NULL,
    ALTER COLUMN count_max SET NOT NULL;

COMMENT ON COLUMN row_template_slots.count_min IS
    'Minimum posts required for this slot. Below this, fill_row abandons. '
    'Flexible rows set count_min < count_max so they emit with variable content.';
COMMENT ON COLUMN row_template_slots.count_max IS
    'Maximum posts this slot will consume. Engine fills up to this count. '
    'Default equals count (exact count) for backward compatibility.';

-- ═══════════════════════════════════════════════════════════════════════
-- Catchall row templates for the spillover phase.
-- Broad accepts arrays + flexible counts + no post_template requirement.
-- The engine picks the best compatible post_template per post.
-- ═══════════════════════════════════════════════════════════════════════

-- catchall-trio-light: 1-3 light posts of any type, uses digest template.
INSERT INTO row_template_configs (slug, display_name, description, layout_variant, sort_order)
VALUES ('catchall-trio-light', 'Catchall Light Trio',
  'Flexible trio of 1-3 light posts, any type. Spillover container for unplaced light content.',
  'trio', 900)
ON CONFLICT (slug) DO NOTHING;
INSERT INTO row_template_slots (row_template_config_id, slot_index, weight, count, count_min, count_max, post_template_slug, accepts)
SELECT id, 0, 'light', 3, 1, 3, NULL,
  ARRAY['story','update','action','event','need','aid','person','business','reference']
FROM row_template_configs WHERE slug='catchall-trio-light'
ON CONFLICT DO NOTHING;

-- catchall-pair-light: 1-2 light posts of any type.
INSERT INTO row_template_configs (slug, display_name, description, layout_variant, sort_order)
VALUES ('catchall-pair-light', 'Catchall Light Pair',
  'Flexible pair of 1-2 light posts, any type. Spillover container for small residue.',
  'pair', 901)
ON CONFLICT (slug) DO NOTHING;
INSERT INTO row_template_slots (row_template_config_id, slot_index, weight, count, count_min, count_max, post_template_slug, accepts)
SELECT id, 0, 'light', 1, 1, 1, NULL,
  ARRAY['story','update','action','event','need','aid','person','business','reference']
FROM row_template_configs WHERE slug='catchall-pair-light'
ON CONFLICT DO NOTHING;
INSERT INTO row_template_slots (row_template_config_id, slot_index, weight, count, count_min, count_max, post_template_slug, accepts)
SELECT id, 1, 'light', 1, 0, 1, NULL,
  ARRAY['story','update','action','event','need','aid','person','business','reference']
FROM row_template_configs WHERE slug='catchall-pair-light'
ON CONFLICT DO NOTHING;

-- catchall-trio-medium: 1-3 medium posts of any type.
INSERT INTO row_template_configs (slug, display_name, description, layout_variant, sort_order)
VALUES ('catchall-trio-medium', 'Catchall Medium Trio',
  'Flexible trio of 1-3 medium posts, any type. Spillover for unplaced medium content.',
  'trio', 902)
ON CONFLICT (slug) DO NOTHING;
INSERT INTO row_template_slots (row_template_config_id, slot_index, weight, count, count_min, count_max, post_template_slug, accepts)
SELECT id, 0, 'medium', 3, 1, 3, 'gazette',
  ARRAY['story','update','action','event','need','aid','person','business','reference']
FROM row_template_configs WHERE slug='catchall-trio-medium'
ON CONFLICT DO NOTHING;

-- catchall-pair-medium: 1-2 medium posts of any type, gazette treatment.
INSERT INTO row_template_configs (slug, display_name, description, layout_variant, sort_order)
VALUES ('catchall-pair-medium', 'Catchall Medium Pair',
  'Flexible pair of 1-2 medium posts, any type.',
  'pair', 903)
ON CONFLICT (slug) DO NOTHING;
INSERT INTO row_template_slots (row_template_config_id, slot_index, weight, count, count_min, count_max, post_template_slug, accepts)
SELECT id, 0, 'medium', 1, 1, 1, 'gazette',
  ARRAY['story','update','action','event','need','aid','person','business','reference']
FROM row_template_configs WHERE slug='catchall-pair-medium'
ON CONFLICT DO NOTHING;
INSERT INTO row_template_slots (row_template_config_id, slot_index, weight, count, count_min, count_max, post_template_slug, accepts)
SELECT id, 1, 'medium', 1, 0, 1, 'gazette',
  ARRAY['story','update','action','event','need','aid','person','business','reference']
FROM row_template_configs WHERE slug='catchall-pair-medium'
ON CONFLICT DO NOTHING;
