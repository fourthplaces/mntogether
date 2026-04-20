-- Denser classifieds trio rows — Part 1 of the density/grouping pass.
--
-- The classifieds-style trios (three equal 2-unit cells, each a small
-- stack of light-weight posts) were sized with 2 posts per cell. At
-- that count each column has only ~4 visual units of content and
-- visually reads as "two posts adrift" rather than a proper classifieds
-- band. Bumping to 3 per cell yields 9 posts total in the row with
-- ~6 units per column — the density the design intends.
--
-- Scope: only the classifieds-family templates (digest / ledger
-- variants). trio-bulletin, trio-whisper, trio-gazette etc. stay at
-- count=1 per cell because their post templates are already tall
-- (bulletin=7, whisper=~7) and don't benefit from stacking.

UPDATE row_template_slots
SET count = 3
WHERE row_template_config_id = (SELECT id FROM row_template_configs WHERE slug = 'classifieds');

UPDATE row_template_slots
SET count = 3
WHERE row_template_config_id = (SELECT id FROM row_template_configs WHERE slug = 'classifieds-ledger');

UPDATE row_template_slots
SET count = 3
WHERE row_template_config_id = (SELECT id FROM row_template_configs WHERE slug = 'classifieds-ledger-alt');

-- Keep descriptions consistent with the new shape.
UPDATE row_template_configs
SET description = 'Three digest columns, three posts each. Classifieds-style band of nine light items.'
WHERE slug = 'classifieds';

UPDATE row_template_configs
SET description = 'Three ledger columns, three posts each. Denser variant for reference-heavy weeks.'
WHERE slug = 'classifieds-ledger';

UPDATE row_template_configs
SET description = 'Alternate ledger trio (three per cell) to vary layout within a single edition.'
WHERE slug = 'classifieds-ledger-alt';
