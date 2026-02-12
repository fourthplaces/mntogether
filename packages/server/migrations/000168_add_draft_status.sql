-- Add status column to notes for draft support.
-- 'draft': consultant-proposed notes awaiting admin review
-- 'active': live notes (all existing notes default to active)
--
-- Posts already have a status column. The consultant will use 'draft' status
-- for proposed posts. Existing 'pending_approval' remains valid for the legacy
-- pipeline during the migration period.

ALTER TABLE notes ADD COLUMN status TEXT NOT NULL DEFAULT 'active';

-- Update the posts status CHECK constraint to include 'draft'.
-- Constraint is named listings_status_check (legacy from table rename).
ALTER TABLE posts DROP CONSTRAINT IF EXISTS listings_status_check;
ALTER TABLE posts ADD CONSTRAINT listings_status_check
    CHECK (status IN ('draft', 'pending_approval', 'active', 'filled', 'rejected', 'expired', 'archived'));
