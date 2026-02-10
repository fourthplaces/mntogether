-- Add unique constraint on organization name for find_or_create dedup
ALTER TABLE organizations ADD CONSTRAINT organizations_name_unique UNIQUE (name);

-- Fix cascade behavior for organization deletion
ALTER TABLE websites DROP CONSTRAINT IF EXISTS websites_organization_id_fkey;
ALTER TABLE websites ADD CONSTRAINT websites_organization_id_fkey
    FOREIGN KEY (organization_id) REFERENCES organizations(id) ON DELETE SET NULL;

ALTER TABLE social_profiles DROP CONSTRAINT IF EXISTS social_profiles_organization_id_fkey;
ALTER TABLE social_profiles ADD CONSTRAINT social_profiles_organization_id_fkey
    FOREIGN KEY (organization_id) REFERENCES organizations(id) ON DELETE CASCADE;
