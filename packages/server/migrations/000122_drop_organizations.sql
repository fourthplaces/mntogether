-- Remove FK constraints referencing organizations
ALTER TABLE posts DROP COLUMN IF EXISTS organization_id;
ALTER TABLE posts DROP COLUMN IF EXISTS organization_name;
ALTER TABLE locations DROP COLUMN IF EXISTS organization_id;

-- Drop org tagging tables
DROP TABLE IF EXISTS tags_on_organizations;

-- Drop the organizations table itself
DROP TABLE IF EXISTS organizations;
