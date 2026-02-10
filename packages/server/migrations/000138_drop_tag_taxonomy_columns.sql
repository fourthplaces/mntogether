-- Drop redundant taxonomy columns from tags table.
-- External taxonomy mapping is handled by the taxonomy_crosswalks table.
ALTER TABLE tags DROP COLUMN IF EXISTS external_code;
ALTER TABLE tags DROP COLUMN IF EXISTS taxonomy_source;
