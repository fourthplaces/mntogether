-- Same principle as migration 223 (fixing pair-stack balance for light/medium
-- pairs), but now for medium+medium pairs. When a pair has two DIFFERENT
-- medium post_templates, the rendered heights diverge — a gazette rendering
-- a story with full body text is far taller than a bulletin rendering a
-- short community request.
--
-- Convert the three mismatched medium+medium pairs to pair-stack variants
-- with count=2 on the shorter-rendering side. The stacked pair balances
-- the taller anchor's rendered height.
--
-- Same-template pairs (pair-spotlight, pair-exchange, pair-alert-notice,
-- pair-bulletin-event) stay as-is — they're naturally balanced.

-- pair-gazette-bulletin: gazette (tall stories/refs) + bulletin (short asks)
-- Convert to pair-stack: 1 gazette + 2 bulletin stacked.
UPDATE row_template_configs
SET layout_variant = 'pair-stack',
    display_name = 'Gazette + Bulletin Stack',
    description = 'One gazette-style medium anchoring, with two bulletin-style mediums stacked alongside to balance the height.'
WHERE slug = 'pair-gazette-bulletin';

UPDATE row_template_slots
SET count = 2, count_max = 2, count_min = 2
WHERE row_template_config_id = (SELECT id FROM row_template_configs WHERE slug='pair-gazette-bulletin')
  AND slot_index = 1;


-- pair-gazette-spotlight: gazette (tall) + spotlight (medium-short)
-- Convert to pair-stack: 1 gazette + 2 spotlight stacked.
UPDATE row_template_configs
SET layout_variant = 'pair-stack',
    display_name = 'Gazette + Spotlight Stack',
    description = 'One gazette-style medium anchoring, with two local spotlights stacked alongside.'
WHERE slug = 'pair-gazette-spotlight';

UPDATE row_template_slots
SET count = 2, count_max = 2, count_min = 2
WHERE row_template_config_id = (SELECT id FROM row_template_configs WHERE slug='pair-gazette-spotlight')
  AND slot_index = 1;


-- pair-spotlight-bulletin: spotlight + bulletin (bulletin often shorter)
-- Convert to pair-stack: 1 spotlight + 2 bulletin stacked.
UPDATE row_template_configs
SET layout_variant = 'pair-stack',
    display_name = 'Spotlight + Bulletin Stack',
    description = 'One local spotlight anchoring, with two bulletin-style mediums stacked alongside.'
WHERE slug = 'pair-spotlight-bulletin';

UPDATE row_template_slots
SET count = 2, count_max = 2, count_min = 2
WHERE row_template_config_id = (SELECT id FROM row_template_configs WHERE slug='pair-spotlight-bulletin')
  AND slot_index = 1;
