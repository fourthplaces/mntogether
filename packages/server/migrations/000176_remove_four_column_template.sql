-- Remove the four-column row template.
-- Design constraint: 3 CSS-grid columns max at the widest breakpoint.
-- Posts span/stack within those 3 columns; a 4-column layout is not possible.

-- 1. Clean up any edition data using this template
DELETE FROM edition_slots
WHERE edition_row_id IN (
    SELECT er.id FROM edition_rows er
    JOIN row_template_configs rtc ON er.row_template_config_id = rtc.id
    WHERE rtc.slug = 'four-column'
);

DELETE FROM edition_rows
WHERE row_template_config_id = (
    SELECT id FROM row_template_configs WHERE slug = 'four-column'
);

-- 2. Remove the template definition
DELETE FROM row_template_slots
WHERE row_template_config_id = (
    SELECT id FROM row_template_configs WHERE slug = 'four-column'
);

DELETE FROM row_template_configs WHERE slug = 'four-column';
