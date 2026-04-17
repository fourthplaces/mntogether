-- Reshape row templates where a single template slot held multiple posts that
-- the broadsheet actually distributes across multiple visual cells. The admin
-- DnD preview couldn't represent this — one template slot = one drop zone,
-- so classifieds etc. showed as a single tall column while the public render
-- laid them out across 3 columns.
--
-- Templates reshaped (all layout_variant = 'trio'):
--   classifieds             1 slot × count=6   →  3 slots × count=2   (digest)
--   three-column            1 slot × count=3   →  3 slots × count=1   (gazette)
--   catchall-trio-light     1 slot × count=3   →  3 slots × count=1   (any light)
--   catchall-trio-medium    1 slot × count=3   →  3 slots × count=1   (any medium)
--
-- For any already-populated edition rows using these templates, remap the
-- slot_index values so posts land in the right visual cells (e.g. for
-- classifieds: six posts all at slot_index=0 become two at 0, two at 1, two
-- at 2, based on created_at order).

-- ---------------------------------------------------------------------------
-- classifieds: 1×6 → 3×2 (digest)
-- ---------------------------------------------------------------------------
DELETE FROM row_template_slots
  WHERE row_template_config_id = (SELECT id FROM row_template_configs WHERE slug = 'classifieds');

INSERT INTO row_template_slots (row_template_config_id, slot_index, weight, count, count_min, count_max, post_template_slug, accepts)
VALUES
  ((SELECT id FROM row_template_configs WHERE slug = 'classifieds'), 0, 'light', 2, 2, 2, 'digest', NULL),
  ((SELECT id FROM row_template_configs WHERE slug = 'classifieds'), 1, 'light', 2, 2, 2, 'digest', NULL),
  ((SELECT id FROM row_template_configs WHERE slug = 'classifieds'), 2, 'light', 2, 2, 2, 'digest', NULL);

-- Remap existing edition_slots for classifieds rows: distribute by created_at
-- into 3 buckets of 2.
WITH ranked AS (
  SELECT es.id,
         ((ROW_NUMBER() OVER (PARTITION BY es.edition_row_id ORDER BY es.created_at, es.id) - 1) / 2)::int AS new_slot_index
  FROM edition_slots es
  JOIN edition_rows er ON er.id = es.edition_row_id
  JOIN row_template_configs rtc ON rtc.id = er.row_template_config_id
  WHERE rtc.slug = 'classifieds'
)
UPDATE edition_slots es
SET slot_index = ranked.new_slot_index
FROM ranked
WHERE es.id = ranked.id;

-- ---------------------------------------------------------------------------
-- three-column: 1×3 → 3×1 (gazette)
-- ---------------------------------------------------------------------------
-- Preserve original post_template_slug by looking it up before delete.
DO $$
DECLARE v_tpl text;
BEGIN
  SELECT post_template_slug INTO v_tpl
    FROM row_template_slots
   WHERE row_template_config_id = (SELECT id FROM row_template_configs WHERE slug = 'three-column')
   LIMIT 1;

  DELETE FROM row_template_slots
    WHERE row_template_config_id = (SELECT id FROM row_template_configs WHERE slug = 'three-column');

  INSERT INTO row_template_slots (row_template_config_id, slot_index, weight, count, count_min, count_max, post_template_slug, accepts)
  VALUES
    ((SELECT id FROM row_template_configs WHERE slug = 'three-column'), 0, 'medium', 1, 1, 1, v_tpl, NULL),
    ((SELECT id FROM row_template_configs WHERE slug = 'three-column'), 1, 'medium', 1, 1, 1, v_tpl, NULL),
    ((SELECT id FROM row_template_configs WHERE slug = 'three-column'), 2, 'medium', 1, 1, 1, v_tpl, NULL);
END $$;

WITH ranked AS (
  SELECT es.id,
         (ROW_NUMBER() OVER (PARTITION BY es.edition_row_id ORDER BY es.created_at, es.id) - 1)::int AS new_slot_index
  FROM edition_slots es
  JOIN edition_rows er ON er.id = es.edition_row_id
  JOIN row_template_configs rtc ON rtc.id = er.row_template_config_id
  WHERE rtc.slug = 'three-column'
)
UPDATE edition_slots es
SET slot_index = ranked.new_slot_index
FROM ranked
WHERE es.id = ranked.id;

-- ---------------------------------------------------------------------------
-- catchall-trio-light: 1×3 → 3×1 (light, any accepts)
-- ---------------------------------------------------------------------------
DO $$
DECLARE v_accepts text[];
BEGIN
  SELECT accepts INTO v_accepts
    FROM row_template_slots
   WHERE row_template_config_id = (SELECT id FROM row_template_configs WHERE slug = 'catchall-trio-light')
   LIMIT 1;

  DELETE FROM row_template_slots
    WHERE row_template_config_id = (SELECT id FROM row_template_configs WHERE slug = 'catchall-trio-light');

  INSERT INTO row_template_slots (row_template_config_id, slot_index, weight, count, count_min, count_max, post_template_slug, accepts)
  VALUES
    ((SELECT id FROM row_template_configs WHERE slug = 'catchall-trio-light'), 0, 'light', 1, 1, 1, NULL, v_accepts),
    ((SELECT id FROM row_template_configs WHERE slug = 'catchall-trio-light'), 1, 'light', 1, 1, 1, NULL, v_accepts),
    ((SELECT id FROM row_template_configs WHERE slug = 'catchall-trio-light'), 2, 'light', 1, 1, 1, NULL, v_accepts);
END $$;

WITH ranked AS (
  SELECT es.id,
         (ROW_NUMBER() OVER (PARTITION BY es.edition_row_id ORDER BY es.created_at, es.id) - 1)::int AS new_slot_index
  FROM edition_slots es
  JOIN edition_rows er ON er.id = es.edition_row_id
  JOIN row_template_configs rtc ON rtc.id = er.row_template_config_id
  WHERE rtc.slug = 'catchall-trio-light'
)
UPDATE edition_slots es
SET slot_index = ranked.new_slot_index
FROM ranked
WHERE es.id = ranked.id;

-- ---------------------------------------------------------------------------
-- catchall-trio-medium: 1×3 → 3×1 (medium, any accepts)
-- ---------------------------------------------------------------------------
DO $$
DECLARE v_accepts text[];
BEGIN
  SELECT accepts INTO v_accepts
    FROM row_template_slots
   WHERE row_template_config_id = (SELECT id FROM row_template_configs WHERE slug = 'catchall-trio-medium')
   LIMIT 1;

  DELETE FROM row_template_slots
    WHERE row_template_config_id = (SELECT id FROM row_template_configs WHERE slug = 'catchall-trio-medium');

  INSERT INTO row_template_slots (row_template_config_id, slot_index, weight, count, count_min, count_max, post_template_slug, accepts)
  VALUES
    ((SELECT id FROM row_template_configs WHERE slug = 'catchall-trio-medium'), 0, 'medium', 1, 1, 1, NULL, v_accepts),
    ((SELECT id FROM row_template_configs WHERE slug = 'catchall-trio-medium'), 1, 'medium', 1, 1, 1, NULL, v_accepts),
    ((SELECT id FROM row_template_configs WHERE slug = 'catchall-trio-medium'), 2, 'medium', 1, 1, 1, NULL, v_accepts);
END $$;

WITH ranked AS (
  SELECT es.id,
         (ROW_NUMBER() OVER (PARTITION BY es.edition_row_id ORDER BY es.created_at, es.id) - 1)::int AS new_slot_index
  FROM edition_slots es
  JOIN edition_rows er ON er.id = es.edition_row_id
  JOIN row_template_configs rtc ON rtc.id = er.row_template_config_id
  WHERE rtc.slug = 'catchall-trio-medium'
)
UPDATE edition_slots es
SET slot_index = ranked.new_slot_index
FROM ranked
WHERE es.id = ranked.id;
