-- 1. Add `is_evergreen` flag to posts.
--    Evergreen posts (references, business directories, recurring events)
--    bypass the layout engine's 7-day published_at filter. Without this,
--    reference posts fall out of eligibility and require a publishedAt hack.
ALTER TABLE posts
    ADD COLUMN is_evergreen boolean NOT NULL DEFAULT false;

COMMENT ON COLUMN posts.is_evergreen IS
    'Evergreen posts bypass the 7-day published_at eligibility filter '
    'in the layout engine. Use for reference directories, business listings, '
    'and other standing content that should always be broadsheet-eligible.';

-- Backfill: mark existing reference and business posts as evergreen.
UPDATE posts SET is_evergreen = true WHERE post_type IN ('reference', 'business');


-- 2. Add `height_override` JSONB column to post_template_configs.
--    Allows per-post-type height adjustments without hardcoding in Rust.
--    Format: {"reference": 6, "business": 5} — overrides height_units
--    when a post of that type renders via this template.
ALTER TABLE post_template_configs
    ADD COLUMN height_override jsonb;

COMMENT ON COLUMN post_template_configs.height_override IS
    'Per-post-type height unit overrides. JSON object mapping post_type '
    'to an integer height. When a post of that type renders via this '
    'template, the override replaces the default height_units. Used for '
    'templates where different types render at substantially different '
    'heights (e.g. ledger + reference = tall items list).';

-- Seed the known outliers (previously hardcoded in effective_height()).
UPDATE post_template_configs
SET height_override = '{"reference": 6}'::jsonb
WHERE slug = 'ledger';

UPDATE post_template_configs
SET height_override = '{"reference": 10}'::jsonb
WHERE slug = 'bulletin';
