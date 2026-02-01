-- Migration: Copy existing listings to new resources table
-- This migrates data from the complex listings model to the simpler resources model.
--
-- The listings table is preserved for backwards compatibility until the transition
-- is complete and the old code paths are removed.

-- Step 1: Migrate active and pending listings to resources
INSERT INTO resources (
    id,
    website_id,
    title,
    content,
    location,
    status,
    organization_name,
    created_at,
    updated_at
)
SELECT
    l.id,  -- Preserve IDs for traceability
    l.website_id,
    l.title,
    COALESCE(l.description_markdown, l.description) as content,
    l.location,
    l.status,
    l.organization_name,
    l.created_at,
    l.updated_at
FROM listings l
WHERE l.website_id IS NOT NULL  -- Only scraped listings have website_id
  AND l.status IN ('pending_approval', 'active')
ON CONFLICT (id) DO NOTHING;  -- Skip if already migrated

-- Step 2: Migrate source URLs from listings to resource_sources
INSERT INTO resource_sources (resource_id, page_url, created_at)
SELECT
    l.id,
    l.source_url,
    l.created_at
FROM listings l
WHERE l.website_id IS NOT NULL
  AND l.source_url IS NOT NULL
  AND l.status IN ('pending_approval', 'active')
  AND EXISTS (SELECT 1 FROM resources r WHERE r.id = l.id)
ON CONFLICT (resource_id, page_url) DO NOTHING;

-- Step 3: Migrate contacts from listing_contacts to contacts (polymorphic)
-- First, migrate contacts that were in the listing_contacts table
INSERT INTO contacts (
    contactable_type,
    contactable_id,
    contact_type,
    contact_value,
    contact_label,
    is_public,
    display_order,
    created_at
)
SELECT
    'resource' as contactable_type,
    lc.listing_id as contactable_id,
    lc.contact_type,
    lc.contact_value,
    lc.contact_label,
    true as is_public,
    COALESCE(lc.display_order, 0) as display_order,
    NOW() as created_at
FROM listing_contacts lc
WHERE EXISTS (SELECT 1 FROM resources r WHERE r.id = lc.listing_id)
ON CONFLICT (contactable_type, contactable_id, contact_type, contact_value) DO NOTHING;

-- Step 4: Migrate tags from taggables (polymorphic) to resource_tags (direct)
INSERT INTO resource_tags (resource_id, tag_id, created_at)
SELECT
    t.taggable_id as resource_id,
    t.tag_id,
    t.added_at as created_at
FROM taggables t
WHERE t.taggable_type = 'listing'
  AND EXISTS (SELECT 1 FROM resources r WHERE r.id = t.taggable_id)
ON CONFLICT (resource_id, tag_id) DO NOTHING;

-- Step 5: Create initial version records for migrated resources
INSERT INTO resource_versions (
    resource_id,
    title,
    content,
    location,
    change_reason,
    created_at
)
SELECT
    r.id,
    r.title,
    r.content,
    r.location,
    'created',
    r.created_at
FROM resources r
WHERE NOT EXISTS (
    SELECT 1 FROM resource_versions rv WHERE rv.resource_id = r.id
);

-- Step 6: Log the migration results
DO $$
DECLARE
    migrated_count INTEGER;
    sources_count INTEGER;
    contacts_count INTEGER;
    tags_count INTEGER;
BEGIN
    SELECT COUNT(*) INTO migrated_count FROM resources;
    SELECT COUNT(*) INTO sources_count FROM resource_sources;
    SELECT COUNT(*) INTO contacts_count FROM contacts WHERE contactable_type = 'resource';
    SELECT COUNT(*) INTO tags_count FROM resource_tags;

    RAISE NOTICE 'Migration complete: % resources, % sources, % contacts, % tags',
        migrated_count, sources_count, contacts_count, tags_count;
END $$;
