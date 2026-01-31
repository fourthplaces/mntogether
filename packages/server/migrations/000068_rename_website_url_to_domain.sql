-- Rename websites.url to websites.domain for semantic correctness
-- The column stores domain names (e.g., "dhhmn.com"), not full URLs

ALTER TABLE websites RENAME COLUMN url TO domain;

-- Update the unique constraint/index if one exists on url
-- (The unique constraint is created implicitly by the ON CONFLICT clause in queries)
