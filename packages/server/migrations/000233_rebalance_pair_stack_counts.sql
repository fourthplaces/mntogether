-- Rebalance pair-stack templates so the stacked cell fills its column.
--
-- The anchor cell in pair-stack / lead-stack layouts renders with the
-- "anchor exemption" from prepare.ts — its body gets the full weight
-- tier (heavy instead of medium) and skips line-clamping. Even after
-- the client-side clamp={6} cap on AlertNotice and GazetteStory, the
-- anchor's rendered height (title + flag + deck + 6 lines of body +
-- meta) still tops ~8–10 visual units. Meanwhile the stacked cell was
-- sized at 3 × digest = 6 declared units — visibly short, leaving a
-- large empty band on the right side of the row.
--
-- Bump stacked counts on the templates whose anchor is ≥ medium weight
-- and has the anchor-exemption body:
--
--   lead-alert-digest    : digest × 2 → digest × 4   (lead-stack, narrow 2-col stack)
--   pair-stack-alert     : digest × 3 → digest × 5   (3+3, wider stack — needs more to match)
--   pair-stack-spotlight : digest × 3 → digest × 5   (3+3, wider stack — needs more to match)
--
-- Templates intentionally NOT changed:
--   pair-bulletin-digest  (bulletin 7 vs 3×digest 6 — right already near-balanced)
--   pair-stack-gazette    (gazette 8 vs 3×ledger 9 — right already slightly taller)
--   pair-gazette-bulletin (right already taller: 2×bulletin = 14)
--   pair-stack-directory  (directory-ref anchor behavior needs separate audit)

UPDATE row_template_slots
SET count = 4
WHERE row_template_config_id = (SELECT id FROM row_template_configs WHERE slug = 'lead-alert-digest')
  AND slot_index = 1;

UPDATE row_template_slots
SET count = 5
WHERE row_template_config_id = (SELECT id FROM row_template_configs WHERE slug = 'pair-stack-alert')
  AND slot_index = 1;

UPDATE row_template_slots
SET count = 5
WHERE row_template_config_id = (SELECT id FROM row_template_configs WHERE slug = 'pair-stack-spotlight')
  AND slot_index = 1;

-- Keep descriptions in sync with the new shape.
UPDATE row_template_configs
SET description = 'One alert-notice anchor with four digest briefs stacked alongside. Fits the anchor-exemption body on the left to a fuller stack on the right.'
WHERE slug = 'lead-alert-digest';

UPDATE row_template_configs
SET description = 'One alert-notice anchor with five digest briefs stacked to match the anchor''s rendered height after line-clamping.'
WHERE slug = 'pair-stack-alert';

UPDATE row_template_configs
SET description = 'One spotlight-local anchor with five digest briefs stacked to match the anchor''s rendered height after line-clamping.'
WHERE slug = 'pair-stack-spotlight';
