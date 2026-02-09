-- Fix taggable_type check constraint to include 'post' (renamed from 'listing' in 000117)
-- and remove stale 'listing' value that no longer exists.

ALTER TABLE taggables DROP CONSTRAINT IF EXISTS taggables_taggable_type_check;

ALTER TABLE taggables ADD CONSTRAINT taggables_taggable_type_check
    CHECK (taggable_type IN ('post', 'organization', 'referral_document', 'domain', 'provider', 'container', 'website', 'resource'));
