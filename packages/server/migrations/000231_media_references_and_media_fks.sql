-- Media Library unification — Phase 1 foundation.
--
-- Until now, images were authored in two disconnected ways:
--   (1) The `media` table + `/admin/media` page (a proper library, with
--       presigned uploads to MinIO) — isolated from everything else.
--   (2) Raw URL text fields on post_media.image_url, post_person.photo_url,
--       widget.data.image, and Plate body photo nodes.
--
-- This migration wires the two worlds together:
--   - Adds a polymorphic `media_references` table so every reference
--     (post hero, post person, post body, widget image, org logo) is
--     tracked uniformly. The Library UI can then show "used by N places"
--     without scanning JSON, and can list consumers in a detail panel.
--   - Adds `media_id` FK columns alongside existing raw-URL fields on
--     post_media and post_person. Raw URL stays as a denormalized read
--     path; `media_id` is the canonical link when the image came from
--     the Library.
--   - Adds `logo_media_id` + `logo_url` to organizations (net-new feature).
--
-- Widget `data` JSONB can't FK directly; widgets track usage via
-- `media_references` rows written on save. Plate body images track usage
-- the same way (walk bodyAst, extract mediaIds, reconcile rows).
--
-- Backfill is intentionally cautious: we only link `media_id` where the
-- existing raw URL exactly matches a known `media.url`. External URLs
-- (e.g. seed data's Unsplash hotlinks) remain NULL — they'll be replaced
-- with locally-hosted images as part of the seed swap in this phase.

-- ---------------------------------------------------------------------------
-- media_references (polymorphic)
-- ---------------------------------------------------------------------------
CREATE TABLE media_references (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  media_id UUID NOT NULL REFERENCES media(id) ON DELETE CASCADE,

  -- One of: 'post_hero', 'post_person', 'post_body', 'widget', 'organization_logo'
  referenceable_type TEXT NOT NULL,

  -- FK to posts.id / widgets.id / organizations.id — no DB constraint
  -- because the target table varies. App cleans up on entity delete.
  referenceable_id UUID NOT NULL,

  -- Disambiguates multiple refs from the same entity (e.g. Plate body can
  -- embed several images; each gets a distinct field_key like
  -- 'plate_image_1', 'plate_image_2'). NULL when the entity has a single
  -- slot for this media (e.g. post_hero, post_person).
  field_key TEXT,

  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

  UNIQUE (media_id, referenceable_type, referenceable_id, field_key)
);

CREATE INDEX idx_media_references_entity
  ON media_references (referenceable_type, referenceable_id);

CREATE INDEX idx_media_references_media
  ON media_references (media_id);

-- ---------------------------------------------------------------------------
-- post_media: add media_id FK
-- ---------------------------------------------------------------------------
ALTER TABLE post_media
  ADD COLUMN media_id UUID REFERENCES media(id) ON DELETE SET NULL;

CREATE INDEX idx_post_media_media_id
  ON post_media (media_id);

-- Best-effort backfill: link post_media rows to media rows when URLs match.
UPDATE post_media pm
SET media_id = m.id
FROM media m
WHERE pm.image_url = m.url
  AND pm.media_id IS NULL;

-- ---------------------------------------------------------------------------
-- post_person: add photo_media_id FK
-- ---------------------------------------------------------------------------
ALTER TABLE post_person
  ADD COLUMN photo_media_id UUID REFERENCES media(id) ON DELETE SET NULL;

CREATE INDEX idx_post_person_photo_media_id
  ON post_person (photo_media_id);

UPDATE post_person pp
SET photo_media_id = m.id
FROM media m
WHERE pp.photo_url = m.url
  AND pp.photo_media_id IS NULL;

-- ---------------------------------------------------------------------------
-- organizations: add logo_media_id + denormalized logo_url
-- ---------------------------------------------------------------------------
ALTER TABLE organizations
  ADD COLUMN logo_media_id UUID REFERENCES media(id) ON DELETE SET NULL,
  ADD COLUMN logo_url TEXT;

CREATE INDEX idx_organizations_logo_media_id
  ON organizations (logo_media_id);

-- ---------------------------------------------------------------------------
-- Seed media_references from the backfilled media_id columns so existing
-- links show up in the usage panel immediately.
-- ---------------------------------------------------------------------------
INSERT INTO media_references (media_id, referenceable_type, referenceable_id, field_key)
SELECT pm.media_id, 'post_hero', pm.post_id, NULL
FROM post_media pm
WHERE pm.media_id IS NOT NULL
ON CONFLICT DO NOTHING;

INSERT INTO media_references (media_id, referenceable_type, referenceable_id, field_key)
SELECT pp.photo_media_id, 'post_person', pp.post_id, NULL
FROM post_person pp
WHERE pp.photo_media_id IS NOT NULL
ON CONFLICT DO NOTHING;
