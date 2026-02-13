ALTER TABLE tags ADD COLUMN color TEXT;

COMMENT ON COLUMN tags.color IS 'Optional hex color for display (e.g., #3b82f6)';
