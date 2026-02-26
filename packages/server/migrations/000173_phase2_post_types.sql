-- Phase 2: Expand Post Types
--
-- Migrates from old 4-type system (service/opportunity/business/professional)
-- to new 6-type system (story/notice/exchange/event/spotlight/reference)
-- per CMS_SYSTEM_SPEC.md.
--
-- Types are form presets, not rigid schemas. The post_type_configs table
-- defines default field groups, weight, and compatible templates per type.
-- All field groups are available on all types — type just sets defaults.

-- ============================================================================
-- 1. New columns on posts
-- ============================================================================

-- Layout weight: what column width this post needs on the broadsheet
ALTER TABLE posts ADD COLUMN IF NOT EXISTS weight TEXT NOT NULL DEFAULT 'medium';

-- Editorial priority: higher = more important = closer to top
ALTER TABLE posts ADD COLUMN IF NOT EXISTS priority INT NOT NULL DEFAULT 0;

-- ============================================================================
-- 2. Post type config table (§11 — "types are config, not architecture")
-- ============================================================================

CREATE TABLE IF NOT EXISTS post_type_configs (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    slug            TEXT UNIQUE NOT NULL,
    display_name    TEXT NOT NULL,
    default_weight  TEXT NOT NULL DEFAULT 'medium',
    default_groups  TEXT[] NOT NULL DEFAULT '{}',
    templates       TEXT[] NOT NULL DEFAULT '{}',
    sort_order      INT NOT NULL DEFAULT 0,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

INSERT INTO post_type_configs (slug, display_name, default_weight, default_groups, templates, sort_order) VALUES
    ('story',     'Story',     'heavy',  ARRAY['media','meta'],                                  ARRAY['feature','gazette','digest'],                                           1),
    ('notice',    'Notice',    'light',  ARRAY['meta','source'],                                 ARRAY['gazette','ledger','bulletin','ticker','digest','feature-reversed'],       2),
    ('exchange',  'Exchange',  'medium', ARRAY['contact','items','status'],                      ARRAY['gazette','ledger','bulletin','ticker'],                                  3),
    ('event',     'Event',     'medium', ARRAY['datetime','location','contact'],                 ARRAY['gazette','ledger','bulletin','ticker','feature'],                        4),
    ('spotlight', 'Spotlight', 'medium', ARRAY['person','media','location','contact'],            ARRAY['feature','gazette','bulletin'],                                         5),
    ('reference', 'Reference', 'medium', ARRAY['items','contact','location','schedule'],          ARRAY['gazette','ledger','bulletin'],                                          6)
ON CONFLICT (slug) DO NOTHING;

-- ============================================================================
-- 3. Field group tables
-- ============================================================================

-- Items: name+detail pairs (Exchange needs/offers, Reference directories)
CREATE TABLE IF NOT EXISTS post_items (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    post_id     UUID NOT NULL REFERENCES posts(id) ON DELETE CASCADE,
    name        TEXT NOT NULL,
    detail      TEXT,
    sort_order  INT NOT NULL DEFAULT 0,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS idx_post_items_post_id ON post_items(post_id);

-- Media: images with caption and credit
CREATE TABLE IF NOT EXISTS post_media (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    post_id     UUID NOT NULL REFERENCES posts(id) ON DELETE CASCADE,
    image_url   TEXT,
    caption     TEXT,
    credit      TEXT,
    sort_order  INT NOT NULL DEFAULT 0,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS idx_post_media_post_id ON post_media(post_id);

-- Person: profile fields for Spotlight type (1:1 with post)
CREATE TABLE IF NOT EXISTS post_person (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    post_id     UUID NOT NULL UNIQUE REFERENCES posts(id) ON DELETE CASCADE,
    name        TEXT,
    role        TEXT,
    bio         TEXT,
    photo_url   TEXT,
    quote       TEXT
);

-- Link: CTA button with optional deadline (1:1 with post)
CREATE TABLE IF NOT EXISTS post_link (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    post_id     UUID NOT NULL UNIQUE REFERENCES posts(id) ON DELETE CASCADE,
    label       TEXT,
    url         TEXT,
    deadline    DATE
);

-- Source attribution: who issued this notice/content (1:1 with post)
CREATE TABLE IF NOT EXISTS post_source_attribution (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    post_id         UUID NOT NULL UNIQUE REFERENCES posts(id) ON DELETE CASCADE,
    source_name     TEXT,
    attribution     TEXT
);

-- Meta: editorial metadata — kicker, byline, timestamps (1:1 with post)
CREATE TABLE IF NOT EXISTS post_meta (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    post_id     UUID NOT NULL UNIQUE REFERENCES posts(id) ON DELETE CASCADE,
    kicker      TEXT,
    byline      TEXT,
    timestamp   TIMESTAMPTZ,
    updated     TEXT
);

-- ============================================================================
-- 4. Seed reserved tags
-- ============================================================================

-- Create a 'reserved' tag kind for system-level behavioral tags
INSERT INTO tag_kinds (id, slug, display_name, description, allowed_resource_types, required, is_public, created_at)
VALUES (
    gen_random_uuid(),
    'reserved',
    'Reserved Tags',
    'System tags that trigger specific visual or behavioral treatment',
    ARRAY['post'],
    false,
    true,
    now()
)
ON CONFLICT (slug) DO NOTHING;

-- Insert reserved tags (per CMS_SYSTEM_SPEC.md §6)
INSERT INTO tags (id, kind, value, display_name, description, color, emoji, created_at) VALUES
    (gen_random_uuid(), 'reserved', 'urgent',    'Urgent',    'High-contrast visual treatment, elevated placement',   '#dc2626', '🔴', now()),
    (gen_random_uuid(), 'reserved', 'recurring', 'Recurring', 'Shows schedule pattern instead of one-off date',       '#7c3aed', '🔄', now()),
    (gen_random_uuid(), 'reserved', 'closed',    'Closed',    'Greyed-out treatment, fulfilled/expired badge',        '#6b7280', '⏹️', now()),
    (gen_random_uuid(), 'reserved', 'need',      'Need',      'Direction indicator: something is needed',             '#b45309', '🤲', now()),
    (gen_random_uuid(), 'reserved', 'aid',       'Aid',       'Direction indicator: something is available',          '#15803d', '🤝', now()),
    (gen_random_uuid(), 'reserved', 'action',    'Action',    'CTA rendering: prominent link button + deadline',      '#2563eb', '📢', now()),
    (gen_random_uuid(), 'reserved', 'person',    'Person',    'Spotlight: render as community member profile',        '#8b5cf6', '👤', now()),
    (gen_random_uuid(), 'reserved', 'business',  'Business',  'Spotlight: render as business/org listing',            '#059669', '🏪', now())
ON CONFLICT DO NOTHING;

-- ============================================================================
-- 5. Migrate existing post_type values
-- ============================================================================

-- Map old types to new types
UPDATE posts SET post_type = 'exchange'  WHERE post_type = 'service';
UPDATE posts SET post_type = 'exchange'  WHERE post_type = 'opportunity';
UPDATE posts SET post_type = 'spotlight' WHERE post_type = 'business';
UPDATE posts SET post_type = 'spotlight' WHERE post_type = 'professional';

-- Set priority from relevance_score where available
UPDATE posts SET priority = COALESCE(relevance_score, 50);

-- Tag existing posts with direction indicators based on old type
-- service → aid (these were offerings/services available)
INSERT INTO taggables (id, tag_id, taggable_type, taggable_id, added_at)
SELECT gen_random_uuid(), t.id, 'post', p.id, now()
FROM posts p
CROSS JOIN tags t
WHERE t.kind = 'reserved' AND t.value = 'aid'
  AND p.post_type = 'exchange'
  AND p.submission_type = 'scraped'
  AND NOT EXISTS (
      SELECT 1 FROM taggables tg
      WHERE tg.tag_id = t.id AND tg.taggable_id = p.id AND tg.taggable_type = 'post'
  );

-- Tag spotlight posts from business with 'business' reserved tag
INSERT INTO taggables (id, tag_id, taggable_type, taggable_id, added_at)
SELECT gen_random_uuid(), t.id, 'post', p.id, now()
FROM posts p
CROSS JOIN tags t
WHERE t.kind = 'reserved' AND t.value = 'business'
  AND p.post_type = 'spotlight'
  AND NOT EXISTS (
      SELECT 1 FROM taggables tg
      WHERE tg.tag_id = t.id AND tg.taggable_id = p.id AND tg.taggable_type = 'post'
  );

-- ============================================================================
-- 6. Clean up old post_type tag kind
-- ============================================================================

-- Remove old post_type taggable associations
DELETE FROM taggables WHERE tag_id IN (
    SELECT id FROM tags WHERE kind = 'post_type'
);

-- Remove old post_type tags
DELETE FROM tags WHERE kind = 'post_type';

-- Remove old post_type tag kind
DELETE FROM tag_kinds WHERE slug = 'post_type';

-- ============================================================================
-- 7. Add CHECK constraint for new types + weight
-- ============================================================================

ALTER TABLE posts ADD CONSTRAINT posts_post_type_check
    CHECK (post_type IN ('story', 'notice', 'exchange', 'event', 'spotlight', 'reference'));

ALTER TABLE posts ADD CONSTRAINT posts_weight_check
    CHECK (weight IN ('heavy', 'medium', 'light'));
