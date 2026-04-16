-- Add `is_seed` flag to posts so the dev seeder can safely identify
-- which posts it owns and delete only those on re-seed. Without this,
-- the seeder's blanket DELETE wipes real user-submitted content.
--
-- Default false — existing posts (real content) are preserved.
-- Seeder sets this to true on all inserts and filters its cleanup
-- DELETE by is_seed = true.

ALTER TABLE posts
    ADD COLUMN is_seed boolean NOT NULL DEFAULT false;

COMMENT ON COLUMN posts.is_seed IS
    'True when this post was inserted by data/seed.mjs. The seeder uses '
    'this flag to safely wipe its own content on re-seed without '
    'touching real user-submitted or production content.';

-- Backfill: any existing posts with submission_type IN ('admin','ingested')
-- that match our seed file's common prefixes are from the seeder. Mark
-- them all true so the next re-seed cleanly wipes them.
-- (In a production database this backfill would be omitted; in dev,
-- everything is seed data.)
UPDATE posts SET is_seed = true WHERE submission_type IN ('admin', 'ingested', 'org_submitted', 'reader_submitted');
