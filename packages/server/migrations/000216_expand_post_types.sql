-- Expand post_type enum from 6 to 9 values, simplify post_status.state,
-- add modifier columns (pencil_mark on posts, pull_quote on post_meta),
-- and rewrite template compatibility for the new type system.
--
-- Mental model: types describe WHAT a post is (single value, drives visual variant);
-- modifiers describe HOW it's emphasized (overlays that layer on top).
--
-- Old types:          story | notice | exchange | event | spotlight | reference
-- New types:  story | update | action | event | need | aid | person | business | reference
--
-- Migration rules:
--   notice + has post_link            → action
--   notice (no post_link)             → update
--   exchange + status='needed'        → need
--   exchange + status='available'     → aid
--   exchange (no status)              → aid (best default)
--   spotlight + has post_person row   → person
--   spotlight (no post_person)        → business
--   story / event / reference         → unchanged

-- ============================================================================
-- Step 1: Drop the old CHECK constraint
-- ============================================================================
ALTER TABLE posts DROP CONSTRAINT IF EXISTS posts_post_type_check;

-- ============================================================================
-- Step 2: Migrate data based on type + field group signals
-- ============================================================================

-- notice → action or update
UPDATE posts SET post_type = 'action'
  WHERE post_type = 'notice'
    AND id IN (SELECT post_id FROM post_link WHERE url IS NOT NULL AND url != '');

UPDATE posts SET post_type = 'update'
  WHERE post_type = 'notice';

-- exchange → need or aid
UPDATE posts SET post_type = 'need'
  WHERE post_type = 'exchange'
    AND id IN (SELECT post_id FROM post_status WHERE state = 'needed');

UPDATE posts SET post_type = 'aid'
  WHERE post_type = 'exchange';

-- spotlight → person or business
UPDATE posts SET post_type = 'person'
  WHERE post_type = 'spotlight'
    AND id IN (SELECT post_id FROM post_person WHERE name IS NOT NULL AND name != '');

UPDATE posts SET post_type = 'business'
  WHERE post_type = 'spotlight';

-- ============================================================================
-- Step 3: Add the new CHECK constraint with 9 values
-- ============================================================================
ALTER TABLE posts ADD CONSTRAINT posts_post_type_check
  CHECK (post_type = ANY (ARRAY['story', 'update', 'action', 'event', 'need', 'aid', 'person', 'business', 'reference']));

-- ============================================================================
-- Step 4: Simplify post_status.state from needed/available to open/closed
-- ============================================================================

-- All existing statuses become 'open' (the need/aid split is now captured by post_type)
UPDATE post_status SET state = 'open' WHERE state IN ('needed', 'available');

-- Add CHECK constraint for the new state values
ALTER TABLE post_status DROP CONSTRAINT IF EXISTS post_status_state_check;
ALTER TABLE post_status ADD CONSTRAINT post_status_state_check
  CHECK (state IS NULL OR state = ANY (ARRAY['open', 'closed']));

-- ============================================================================
-- Step 5: Add pencil_mark column for editorial emphasis overlays
-- ============================================================================
ALTER TABLE posts ADD COLUMN pencil_mark text DEFAULT NULL;
ALTER TABLE posts ADD CONSTRAINT posts_pencil_mark_check
  CHECK (pencil_mark IS NULL OR pencil_mark = ANY (ARRAY['star', 'heart', 'smile', 'circle']));

-- ============================================================================
-- Step 6: Add pull_quote column to post_meta for FeatureStory pullQuote rendering
-- ============================================================================
ALTER TABLE post_meta ADD COLUMN pull_quote text DEFAULT NULL;

-- ============================================================================
-- Step 7: Rewrite post_template_configs.compatible_types for the new type system
-- ============================================================================

UPDATE post_template_configs SET compatible_types = ARRAY['story', 'event', 'person']
  WHERE slug = 'feature';

UPDATE post_template_configs SET compatible_types = ARRAY['action', 'update']
  WHERE slug = 'feature-reversed';

UPDATE post_template_configs SET compatible_types = ARRAY['story', 'update', 'action', 'event', 'need', 'aid', 'person', 'business', 'reference']
  WHERE slug = 'gazette';

UPDATE post_template_configs SET compatible_types = ARRAY['update', 'action', 'event', 'need', 'aid', 'business', 'reference']
  WHERE slug = 'bulletin';

UPDATE post_template_configs SET compatible_types = ARRAY['update', 'action', 'event', 'need', 'aid', 'reference']
  WHERE slug = 'ledger';

UPDATE post_template_configs SET compatible_types = ARRAY['story', 'update', 'action', 'need', 'aid']
  WHERE slug = 'digest';

UPDATE post_template_configs SET compatible_types = ARRAY['update', 'action', 'event', 'need', 'aid']
  WHERE slug = 'ticker';

UPDATE post_template_configs SET compatible_types = ARRAY['update', 'action']
  WHERE slug = 'alert-notice';

UPDATE post_template_configs SET compatible_types = ARRAY['need', 'aid']
  WHERE slug = 'pinboard-exchange';

UPDATE post_template_configs SET compatible_types = ARRAY['need', 'aid']
  WHERE slug = 'generous-exchange';

UPDATE post_template_configs SET compatible_types = ARRAY['event']
  WHERE slug = 'card-event';

UPDATE post_template_configs SET compatible_types = ARRAY['reference']
  WHERE slug = 'directory-ref';

UPDATE post_template_configs SET compatible_types = ARRAY['reference']
  WHERE slug = 'quick-ref';

UPDATE post_template_configs SET compatible_types = ARRAY['update']
  WHERE slug = 'whisper-notice';

UPDATE post_template_configs SET compatible_types = ARRAY['person', 'business']
  WHERE slug = 'spotlight-local';

UPDATE post_template_configs SET compatible_types = ARRAY['update']
  WHERE slug = 'ticker-update';

-- ============================================================================
-- Step 8: Fix row template slot `accepts` arrays that reference old types
-- ============================================================================

-- pair-stack-gazette anchor was restricted to story|notice (now story|update)
UPDATE row_template_slots SET accepts = ARRAY['story', 'update']
  WHERE row_template_config_id = (SELECT id FROM row_template_configs WHERE slug = 'pair-stack-gazette')
    AND slot_index = 0;
