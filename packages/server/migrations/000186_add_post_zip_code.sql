-- Add zip_code column to posts.
-- Root Signal provides zip_code; county is inferred from zip_counties table.
ALTER TABLE posts ADD COLUMN zip_code TEXT;
