-- Add 'archived' to allowed post statuses
ALTER TABLE posts DROP CONSTRAINT IF EXISTS listings_status_check;
ALTER TABLE posts ADD CONSTRAINT listings_status_check
  CHECK (status IN ('pending_approval', 'active', 'filled', 'rejected', 'expired', 'archived'));
