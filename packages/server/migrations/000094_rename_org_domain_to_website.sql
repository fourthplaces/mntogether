-- Rename domain_id to website_id in organizations table
-- This was missed in migration 000057 which renamed domains to websites

-- Step 1: Rename the column
ALTER TABLE organizations RENAME COLUMN domain_id TO website_id;

-- Step 2: Drop and recreate the foreign key with new name
ALTER TABLE organizations DROP CONSTRAINT IF EXISTS organizations_domain_id_fkey;

ALTER TABLE organizations
    ADD CONSTRAINT organizations_website_id_fkey
    FOREIGN KEY (website_id) REFERENCES websites(id) ON DELETE SET NULL;

-- Step 3: Rename index
DROP INDEX IF EXISTS idx_organizations_domain;
CREATE INDEX idx_organizations_website ON organizations(website_id);
