-- Add Unique Constraint on content_hash
--
-- CRITICAL: Prevents duplicate need submissions while allowing re-submission of expired content.
--
-- This migration adds a partial unique index that only applies to active and pending_approval needs.
-- Rejected and expired needs are excluded, allowing the same content to be re-submitted in the future.

-- Add partial unique index for organization_needs
-- Only enforces uniqueness for active/pending needs with non-null content_hash
CREATE UNIQUE INDEX idx_organization_needs_content_hash_unique
    ON organization_needs(content_hash)
    WHERE status IN ('pending_approval', 'active')
      AND content_hash IS NOT NULL;

-- Add comment documenting the constraint
COMMENT ON INDEX idx_organization_needs_content_hash_unique IS
    'Prevents duplicate needs during active/pending states. Rejected/expired needs can be resubmitted.';

-- Note: This prevents the race condition where two scrapers simultaneously
-- process the same content and create duplicate pending needs.
