-- Retire pending_approval status: Root Signal posts arrive as 'active',
-- human-authored posts use 'draft'. No posts should be pending_approval.
--
-- Keep 'pending_approval' in the CHECK constraint for backward compatibility,
-- but migrate all existing pending_approval posts to active.

UPDATE posts SET status = 'active', updated_at = NOW()
WHERE status = 'pending_approval';
