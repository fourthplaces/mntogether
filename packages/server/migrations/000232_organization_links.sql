-- Organization Links — dedicated links table, replacing the Platform tag kind
-- as the mechanism for "this org has a Facebook page" etc.
--
-- Background
-- ----------
-- Until now, platform presence was modeled as a `platform` tag kind (46
-- locked values: instagram, facebook, substack, etc.) attached to an org via
-- `tags_on_organizations`. That only recorded _existence_ — there was no URL,
-- no per-link visibility flag, no ordering. Editors could say "this org is on
-- Instagram" but couldn't say _which_ Instagram, and couldn't hide a link
-- from the public profile page (important for individuals sourced from the
-- CMS whose platform presence is operational-only, not for public display).
--
-- This migration introduces `organization_links` as the first-class home for
-- that data. The `platform` tag kind and its 46 `tags` rows stay in place —
-- they become a read-only _lookup_ for the Links picker UI (display name,
-- emoji, color all live on `tags`). We just strip `organization` from its
-- `allowed_resource_types` so the tag UI stops offering platform as an
-- org-taggable kind, and we delete any existing platform-kind tag
-- attachments on orgs.

-- ---------------------------------------------------------------------------
-- organization_links
-- ---------------------------------------------------------------------------
CREATE TABLE organization_links (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,

  -- Platform slug — matches `tags.value` where `tags.kind = 'platform'`.
  -- Stored as plain TEXT (not a FK) so we can accept legacy values and so
  -- deleting a platform lookup row doesn't nuke orgs' historical links.
  platform TEXT NOT NULL,

  url TEXT NOT NULL,

  -- Per-link visibility. Default is TRUE at the DB level; the handler
  -- overrides to FALSE for individual-typed sources at create time, so
  -- individuals' links ship private by default unless an editor opts in.
  is_public BOOLEAN NOT NULL DEFAULT TRUE,

  -- Stable ordering for the Links editor + public render.
  display_order INTEGER NOT NULL DEFAULT 0,

  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_organization_links_organization_id
  ON organization_links (organization_id);

CREATE INDEX idx_organization_links_platform
  ON organization_links (platform);

-- ---------------------------------------------------------------------------
-- Strip `organization` from the platform tag kind's allowed_resource_types
-- so the tag UI no longer offers platform as an org-attachable kind. The
-- row stays (the 46 tags rows it anchors remain a lookup for the Links
-- picker); we just turn off the "you may attach this to orgs" flag.
-- ---------------------------------------------------------------------------
UPDATE tag_kinds
SET allowed_resource_types = '{}'
WHERE slug = 'platform';

-- ---------------------------------------------------------------------------
-- Remove any existing platform-kind tag attachments on orgs. These were
-- dead data anyway (no URL, no visibility) and the Links editor replaces
-- the authoring surface. Tag *definitions* (the 46 rows in `tags`) stay
-- for the Links picker lookup.
--
-- Platform attachments live in the polymorphic `taggables` table (not
-- `organization_tags`, which has a hard CHECK constraint restricting it
-- to service/language/community kinds).
-- ---------------------------------------------------------------------------
DELETE FROM taggables tg
USING tags t
WHERE tg.tag_id = t.id
  AND tg.taggable_type = 'organization'
  AND t.kind = 'platform';
