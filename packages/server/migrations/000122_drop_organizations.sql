-- Drop view that depends on posts.organization_name
DROP VIEW IF EXISTS listing_reports_with_details;

-- Drop tables that FK to organizations
DROP TABLE IF EXISTS business_organizations;

-- Drop stale search function that references organizations
DROP FUNCTION IF EXISTS search_organizations_by_similarity(vector, double precision, integer);

-- Remove FK constraints referencing organizations
ALTER TABLE posts DROP COLUMN IF EXISTS organization_id;
ALTER TABLE posts DROP COLUMN IF EXISTS organization_name;
ALTER TABLE locations DROP COLUMN IF EXISTS organization_id;

-- Drop org tagging tables
DROP TABLE IF EXISTS tags_on_organizations;

-- Drop the organizations table itself
DROP TABLE IF EXISTS organizations CASCADE;
