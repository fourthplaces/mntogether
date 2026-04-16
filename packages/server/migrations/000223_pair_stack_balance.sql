-- Fix fundamentally imbalanced pair row templates.
--
-- A `pair` layout with count=1 in each cell forces both cells to render
-- at their natural heights. When one cell is medium (bulletin, height=7)
-- and the other is light (ledger=3, digest=2), the light cell renders
-- with massive empty space below it because cell heights don't balance
-- across pair layouts (only within stacked cells).
--
-- The correct shape for a medium+light row is `pair-stack`: 1 medium
-- anchor + N lights stacked on the other side to match the anchor's
-- height (as pair-stack-gazette already does).
--
-- Rule going forward: any pair layout with count=1 per cell must use
-- same-weight slots. Cross-weight pairs should be pair-stack variants.

-- Convert pair-bulletin-ledger: 1m bulletin + 1l ledger → 1m bulletin + 3l ledgers.
UPDATE row_template_configs
SET layout_variant = 'pair-stack',
    display_name = 'Bulletin + Ledger Stack',
    description = 'One medium bulletin alongside three light ledgers stacked to match height. Mixes reference/business medium with update/reference lights.'
WHERE slug = 'pair-bulletin-ledger';

UPDATE row_template_slots
SET count = 3
WHERE row_template_config_id = (SELECT id FROM row_template_configs WHERE slug = 'pair-bulletin-ledger')
  AND slot_index = 1;

-- Convert pair-bulletin-digest similarly.
UPDATE row_template_configs
SET layout_variant = 'pair-stack',
    display_name = 'Bulletin + Digest Stack',
    description = 'One medium bulletin alongside three light digests stacked to match height. Mixes reference/business medium with story/update lights.'
WHERE slug = 'pair-bulletin-digest';

UPDATE row_template_slots
SET count = 3
WHERE row_template_config_id = (SELECT id FROM row_template_configs WHERE slug = 'pair-bulletin-digest')
  AND slot_index = 1;
