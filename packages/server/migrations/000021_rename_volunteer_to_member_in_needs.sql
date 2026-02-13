-- Rename volunteer references to member in organization_needs table
-- The volunteers → members table rename was done in 20260127000013,
-- but column references still use the old name

-- Rename the foreign key column
ALTER TABLE organization_needs
    RENAME COLUMN submitted_by_volunteer_id TO submitted_by_member_id;

-- Drop and recreate the index with new name
DROP INDEX IF EXISTS idx_needs_submitted_by_volunteer;
CREATE INDEX idx_needs_submitted_by_member
    ON organization_needs(submitted_by_member_id)
    WHERE submitted_by_member_id IS NOT NULL;

-- Update comment
COMMENT ON COLUMN organization_needs.submitted_by_member_id IS 'Member who submitted this need (for user-submitted needs only)';
