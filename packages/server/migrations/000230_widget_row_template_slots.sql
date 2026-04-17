-- Give widget-only row templates explicit slot definitions.
--
-- widget-standalone, widget-pair, and widget-trio existed as row_template_configs
-- but had no row_template_slots entries. The admin edition editor iterates
-- template slots to render visual cells; with zero template slots, these rows
-- rendered as an empty row even when widgets were assigned — the widgets were
-- in edition_slots but had no template slot to map into.
--
-- Backfill minimal slot definitions so the admin can render them. `accepts`
-- is left NULL and `post_template_slug` is NULL since widget rows hold
-- widgets, not post templates — the existing admin doesn't enforce slot
-- acceptance rules, and the public renderer special-cases widget layouts
-- via BroadsheetRenderer's widget-kind checks.

INSERT INTO row_template_slots (row_template_config_id, slot_index, weight, count, count_min, count_max, post_template_slug, accepts)
VALUES
  ((SELECT id FROM row_template_configs WHERE slug = 'widget-standalone'), 0, 'medium', 1, 1, 1, NULL, NULL),

  ((SELECT id FROM row_template_configs WHERE slug = 'widget-pair'), 0, 'medium', 1, 1, 1, NULL, NULL),
  ((SELECT id FROM row_template_configs WHERE slug = 'widget-pair'), 1, 'medium', 1, 1, 1, NULL, NULL),

  ((SELECT id FROM row_template_configs WHERE slug = 'widget-trio'), 0, 'medium', 1, 1, 1, NULL, NULL),
  ((SELECT id FROM row_template_configs WHERE slug = 'widget-trio'), 1, 'medium', 1, 1, 1, NULL, NULL),
  ((SELECT id FROM row_template_configs WHERE slug = 'widget-trio'), 2, 'medium', 1, 1, 1, NULL, NULL);
