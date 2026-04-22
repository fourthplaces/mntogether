-- Convert `trio-mixed-ref` from a trio (quick-ref + digest + ledger)
-- to a proper pair of references (quick-ref + ledger).
--
-- The original template slotted a `digest` in the middle cell, which
-- meant the layout engine kept depositing a random news item between
-- two resource cards — producing the "lone dig-update marooned between
-- two references" shape flagged in editorial review. Pairing the two
-- reference templates directly (3+3 span instead of 2+2+2) reads as a
-- deliberate resource band and frees that digest to cluster with its
-- peers upstream.
--
-- Existing edition_rows that still reference this template keep their
-- three committed edition_slots. On re-render, the frontend's
-- distributeSlots helper puts overflow into the last cell — so a
-- pre-migration row lands as "quick-ref | digest + ledger stacked".
-- That's an acceptable fallback; regenerating affected editions
-- produces the clean 2-cell layout going forward.

-- Reshape the template.
UPDATE row_template_configs
SET layout_variant = 'pair',
    display_name   = 'Reference Pair',
    description    = 'Two reference cards side-by-side (quick-ref + ledger). Replaces the old trio-mixed-ref whose middle slot accepted any digest and often orphaned a news item between two resource cards.'
WHERE slug = 'trio-mixed-ref';

-- Drop the middle slot (slot_index = 1 was the digest).
DELETE FROM row_template_slots
WHERE row_template_config_id = (SELECT id FROM row_template_configs WHERE slug = 'trio-mixed-ref')
  AND slot_index = 1;

-- Renumber the former slot_index=2 (ledger) down to slot_index=1 so
-- the template's slots are contiguous 0..1 for the new pair layout.
UPDATE row_template_slots
SET slot_index = 1
WHERE row_template_config_id = (SELECT id FROM row_template_configs WHERE slug = 'trio-mixed-ref')
  AND slot_index = 2;
