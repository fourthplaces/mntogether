-- =============================================================================
-- 000237: Post-pivot tag taxonomy scrub
-- =============================================================================
-- Scrubs the pre-pivot tag kinds that migrations 189, 197, and 213 began
-- dismantling but never finished. After this migration the tags table accepts
-- only the post-pivot canonical kinds:
--
--   topic          — open vocabulary (admin-expandable)
--   service_area   — 87 MN counties + statewide (closed)
--   safety         — access-policy modifiers (reserved vocabulary)
--   neighborhood   — reserved for future use (no live rows yet)
--   platform       — preserved; used as a read-only lookup table by the
--                    organization_links picker (display_name, color, emoji).
--                    See migration 232 for the design rationale.
--
-- See docs/architecture/DATA_MODEL.md §5 and docs/status/POST_PIVOT_SCRUB.md §C.
-- =============================================================================

BEGIN;

-- ---------------------------------------------------------------------------
-- 1. Drop dead tag_kinds rows. tag_kinds has no FK into tags, so this just
--    cleans up the metadata table. The actual tag rows are scrubbed below.
-- ---------------------------------------------------------------------------
DELETE FROM tag_kinds
WHERE slug NOT IN (
    'topic',
    'service_area',
    'safety',
    'neighborhood',
    'platform'
);

-- ---------------------------------------------------------------------------
-- 2. Delete dead tags rows. taggables has ON DELETE CASCADE on tag_id so
--    any stale attachments clean up automatically.
-- ---------------------------------------------------------------------------
DELETE FROM tags
WHERE kind NOT IN (
    'topic',
    'service_area',
    'safety',
    'neighborhood',
    'platform'
);

-- ---------------------------------------------------------------------------
-- 3. Ensure service_area has a tag_kinds row. Migration 197 dropped it
--    thinking the kind would be split into county/city; the pivot kept
--    service_area instead. Re-seed so tag_kinds.is_public = true joins work
--    again for public-facing queries.
-- ---------------------------------------------------------------------------
INSERT INTO tag_kinds (slug, display_name, description, allowed_resource_types, required, is_public, locked)
VALUES (
    'service_area',
    'Service Area',
    'Geographic service area — 87 Minnesota counties or statewide.',
    ARRAY['post', 'organization'],
    true,
    true,
    true
)
ON CONFLICT (slug) DO UPDATE SET
    display_name = EXCLUDED.display_name,
    description = EXCLUDED.description,
    allowed_resource_types = EXCLUDED.allowed_resource_types,
    required = EXCLUDED.required,
    is_public = EXCLUDED.is_public,
    locked = EXCLUDED.locked;

-- ---------------------------------------------------------------------------
-- 4. Reserve the neighborhood kind. No tags rows yet — this is a design
--    reservation so future use doesn't collide with unrelated conventions.
--    See DATA_MODEL.md §5.2.
-- ---------------------------------------------------------------------------
INSERT INTO tag_kinds (slug, display_name, description, allowed_resource_types, required, is_public, locked)
VALUES (
    'neighborhood',
    'Neighborhood',
    'Reserved. Sub-county geography currently lives in posts.location, not tags.',
    ARRAY['post', 'organization'],
    false,
    false,
    true
)
ON CONFLICT (slug) DO NOTHING;

-- ---------------------------------------------------------------------------
-- 5. Normalize safety tag slugs to hyphen-case. Drop know_your_rights —
--    per the 2026-04-22 design decisions, "know your rights" is a topic
--    concern, not an access-policy modifier.
-- ---------------------------------------------------------------------------
UPDATE tags SET value = 'no-id-required'
 WHERE kind = 'safety' AND value = 'no_id_required';

UPDATE tags SET value = 'ice-safe'
 WHERE kind = 'safety' AND value = 'ice_safe';

DELETE FROM tags
 WHERE kind = 'safety' AND value = 'know_your_rights';

-- ---------------------------------------------------------------------------
-- 6. Seed the expanded safety vocabulary from TAG_VOCABULARY.md §3.
--    29 access-policy modifier slugs. ON CONFLICT DO UPDATE so running
--    this migration against a database that already has (hyphen-case)
--    rows rewrites the display_name/description to the canonical copy.
-- ---------------------------------------------------------------------------
INSERT INTO tags (kind, value, display_name, description) VALUES
    ('safety', 'no-id-required',          'No ID Required',          'No identification is asked for or required to receive the service.'),
    ('safety', 'immigration-status-safe', 'Immigration Status Safe', 'Will not ask about, record, or report immigration status.'),
    ('safety', 'ice-safe',                'ICE Safe',                'Sanctuary posture; will not voluntarily cooperate with ICE.'),
    ('safety', 'anonymous-access',        'Anonymous Access',        'No name or identifying information required.'),
    ('safety', 'no-background-check',     'No Background Check',     'No criminal-record or background check required.'),
    ('safety', 'free-service',            'Free Service',            'Genuinely no-cost.'),
    ('safety', 'sliding-scale',           'Sliding Scale',           'Fees adjusted based on ability to pay; no one turned away.'),
    ('safety', 'no-insurance-required',   'No Insurance Required',   'Services provided regardless of insurance status.'),
    ('safety', 'confidential',            'Confidential',            'Visits and information kept confidential without explicit consent to share.'),
    ('safety', 'trauma-informed',         'Trauma-Informed',         'Staff trained in trauma-informed care.'),
    ('safety', 'walk-in',                 'Walk-In',                 'No appointment required; drop in during listed hours.'),
    ('safety', 'no-referral-required',    'No Referral Required',    'No referral needed to be seen.'),
    ('safety', 'same-day-service',        'Same-Day Service',        'Seen the same day you arrive.'),
    ('safety', 'women-only',              'Women-Only',              'Space or service restricted to women.'),
    ('safety', 'lgbtq-affirming',         'LGBTQ+ Affirming',         'Staff trained, intake forms inclusive.'),
    ('safety', 'trans-affirming',         'Trans-Affirming',          'Correct name/pronoun use; no gatekeeping.'),
    ('safety', 'indigenous-led',          'Indigenous-Led',          'Led by and for Indigenous community.'),
    ('safety', 'peer-led',                'Peer-Led',                'Delivered by people with lived experience of the issue.'),
    ('safety', 'survivor-centered',       'Survivor-Centered',       'Designed with and for survivors of DV, SA, or trafficking.'),
    ('safety', 'secular',                 'Secular',                 'No religious component, requirement, or expectation.'),
    ('safety', 'disability-accessible',   'Disability-Accessible',   'ADA-compliant physical access.'),
    ('safety', 'asl-available',           'ASL Available',           'American Sign Language interpretation available.'),
    ('safety', 'sensory-friendly',        'Sensory-Friendly',        'Environment designed for sensory-processing needs.'),
    ('safety', 'language-accessible',     'Language-Accessible',     'Interpreters available or multilingual staff on duty.'),
    ('safety', 'harm-reduction',          'Harm Reduction',          'Non-judgmental regardless of substance use.'),
    ('safety', 'minors-without-parent',   'Minors Without Parent',   'Minors can access without parental presence.'),
    ('safety', 'no-law-enforcement',      'No Law Enforcement',      'Does not involve police as policy.'),
    ('safety', 'childcare-provided',      'Childcare Provided',      'Free on-site childcare during the service.'),
    ('safety', 'pets-welcome',            'Pets Welcome',            'Pets or service animals permitted.')
ON CONFLICT (kind, value) DO UPDATE SET
    display_name = EXCLUDED.display_name,
    description  = EXCLUDED.description;

-- ---------------------------------------------------------------------------
-- 7. Constrain tags.kind so future writes can't re-introduce dead kinds.
--    Steps 1–2 already cleared any rows that would violate this.
-- ---------------------------------------------------------------------------
ALTER TABLE tags
    ADD CONSTRAINT tags_kind_check
    CHECK (kind IN (
        'topic',
        'service_area',
        'safety',
        'neighborhood',
        'platform'
    ));

COMMIT;
