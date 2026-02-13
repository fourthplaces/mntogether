-- Remove 'government' from organization_type enum
-- Simplify to: nonprofit, business, community, other

-- Step 1: Check if any organizations are currently using 'government' type
DO $$
DECLARE
  govt_count INTEGER;
BEGIN
  SELECT COUNT(*) INTO govt_count
  FROM organizations
  WHERE organization_type = 'government';

  IF govt_count > 0 THEN
    -- Migrate government orgs to 'other' type
    UPDATE organizations
    SET organization_type = 'other'
    WHERE organization_type = 'government';

    RAISE NOTICE 'Migrated % government organizations to "other" type', govt_count;
  END IF;
END $$;

-- Step 2: Drop the old constraint
ALTER TABLE organizations
  DROP CONSTRAINT IF EXISTS organizations_organization_type_check;

-- Step 3: Add new constraint without 'government'
ALTER TABLE organizations
  ADD CONSTRAINT organizations_organization_type_check
  CHECK (organization_type IN ('nonprofit', 'business', 'community', 'other'));

COMMENT ON COLUMN organizations.organization_type IS 'Organization type: nonprofit | business | community | other';
