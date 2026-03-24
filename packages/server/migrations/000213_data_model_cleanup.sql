-- Phase 0: Data model cleanup
-- Kill dead/redundant columns, fix types, remove unused tag kinds

-- ============================================================
-- 1. Drop dead columns from posts
-- ============================================================

-- Dead: comments feature was removed
ALTER TABLE posts DROP COLUMN IF EXISTS comments_container_id;

-- Dead: generated column, nothing reads it
ALTER TABLE posts DROP COLUMN IF EXISTS title_normalized;

-- Dead: 0 rows populated, post_status.verified is the real one
ALTER TABLE posts DROP COLUMN IF EXISTS verified_at;

-- Redundant with topic tags
ALTER TABLE posts DROP COLUMN IF EXISTS category;

-- Redundant: public source → post_source_attribution field group,
-- raw citations → post_sources (admin provenance)
ALTER TABLE posts DROP COLUMN IF EXISTS source_url;

-- ============================================================
-- 2. Fix urgency: freeform text → boolean flag
-- ============================================================

ALTER TABLE posts DROP COLUMN IF EXISTS urgency;
ALTER TABLE posts ADD COLUMN is_urgent boolean NOT NULL DEFAULT false;

-- ============================================================
-- 3. Fix extraction_confidence: text → integer (0-100)
-- ============================================================

ALTER TABLE posts DROP COLUMN IF EXISTS extraction_confidence;
ALTER TABLE posts ADD COLUMN extraction_confidence integer
  CHECK (extraction_confidence BETWEEN 0 AND 100);

-- ============================================================
-- 4. Normalize submission_type values
-- ============================================================

-- Drop old constraint that only allows 'scraped', 'admin', 'org_submitted', 'revision'
ALTER TABLE posts DROP CONSTRAINT IF EXISTS listings_submission_type_check;

-- Rename scraped → ingested before adding new constraint
UPDATE posts SET submission_type = 'ingested' WHERE submission_type = 'scraped';

-- Add new constraint with updated values
ALTER TABLE posts ADD CONSTRAINT posts_submission_type_check
  CHECK (submission_type = ANY (ARRAY['ingested', 'admin', 'org_submitted', 'reader_submitted', 'revision']));

ALTER TABLE posts ALTER COLUMN submission_type SET DEFAULT 'ingested';

-- ============================================================
-- 5. Kill unused tag kinds
-- ============================================================

-- reserved: only 'urgent' mattered, now replaced by is_urgent column
DELETE FROM taggables WHERE tag_id IN (SELECT id FROM tags WHERE kind = 'reserved');
DELETE FROM tags WHERE kind = 'reserved';

-- structure: pointless fixed list on orgs, replaced by open-ended org tags
DELETE FROM taggables WHERE tag_id IN (SELECT id FROM tags WHERE kind = 'structure');
DELETE FROM tags WHERE kind = 'structure';

-- audience_role: too vague, not useful
DELETE FROM taggables WHERE tag_id IN (SELECT id FROM tags WHERE kind = 'audience_role');
DELETE FROM tags WHERE kind = 'audience_role';
