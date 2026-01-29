-- Add Unique Constraint on content_hash
--
-- CRITICAL: Prevents duplicate listing submissions while allowing re-submission of expired content.
--
-- This migration adds a partial unique index that only applies to active and pending_approval listings.
-- Rejected and expired listings are excluded, allowing the same content to be re-submitted in the future.

-- Add partial unique index for listings
-- Only enforces uniqueness for active/pending listings with non-null content_hash
CREATE UNIQUE INDEX idx_listings_content_hash_unique
    ON listings(content_hash)
    WHERE status IN ('pending_approval', 'active')
      AND content_hash IS NOT NULL;

-- Add comment documenting the constraint
COMMENT ON INDEX idx_listings_content_hash_unique IS
    'Prevents duplicate listings during active/pending states. Rejected/expired listings can be resubmitted.';

-- Note: This prevents the race condition where two scrapers simultaneously
-- process the same content and create duplicate pending needs.
