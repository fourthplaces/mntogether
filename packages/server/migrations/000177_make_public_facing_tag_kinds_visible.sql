-- Make public-facing tag kinds visible to the web app
--
-- The public posts GraphQL resolver joins through tag_kinds with
-- `is_public = true`. Only `reserved` was set to public in migration 000173.
-- This adds the missing tag kinds and marks the ones that should appear
-- on post cards as public.

-- Insert missing tag kinds that exist in tags but not in tag_kinds
INSERT INTO tag_kinds (slug, display_name, description, is_public)
VALUES
    ('topic',        'Topic',        'Content topic (food, housing, health, etc.)',          true),
    ('safety',       'Safety',       'Safety info (no ID required, confidential, etc.)',     true),
    ('certification','Certification','Org certification or accreditation',                   false),
    ('listing_type', 'Listing Type', 'Legacy listing type classification',                  false),
    ('ownership',    'Ownership',    'Business ownership type',                              false),
    ('worker_structure','Worker Structure','How the org staffs its work',                    false)
ON CONFLICT (slug) DO NOTHING;

-- Mark service_area as public (already exists in tag_kinds but is_public = false)
UPDATE tag_kinds SET is_public = true WHERE slug = 'service_area';

-- Mark topic as public (in case the INSERT above hit a conflict)
UPDATE tag_kinds SET is_public = true WHERE slug = 'topic';

-- Mark safety as public
UPDATE tag_kinds SET is_public = true WHERE slug = 'safety';
