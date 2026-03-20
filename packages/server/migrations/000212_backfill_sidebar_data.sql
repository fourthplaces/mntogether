-- Backfill sidebar field-group data so every post type has a populated
-- right sidebar on detail pages.  All INSERTs are idempotent (ON CONFLICT DO NOTHING).

BEGIN;

-- ============================================================================
-- 1. CONTACTS — phone, email, website for posts that don't have any yet
-- ============================================================================

-- Events: add a phone contact
INSERT INTO contacts (contactable_type, contactable_id, contact_type, contact_value, contact_label, display_order)
SELECT 'post', p.id, 'phone',
    CASE (ROW_NUMBER() OVER (ORDER BY p.id)) % 5
        WHEN 0 THEN '(763) 422-7075'
        WHEN 1 THEN '(651) 291-8427'
        WHEN 2 THEN '(612) 348-8570'
        WHEN 3 THEN '(952) 938-4880'
        WHEN 4 THEN '(763) 784-3117'
    END,
    'Event Info',
    0
FROM posts p
WHERE p.post_type = 'event'
  AND p.status = 'active'
  AND p.deleted_at IS NULL
  AND NOT EXISTS (SELECT 1 FROM contacts c WHERE c.contactable_type = 'post' AND c.contactable_id = p.id)
ON CONFLICT DO NOTHING;

-- Exchanges: phone + email
INSERT INTO contacts (contactable_type, contactable_id, contact_type, contact_value, contact_label, display_order)
SELECT 'post', p.id, 'phone',
    CASE (ROW_NUMBER() OVER (ORDER BY p.id)) % 4
        WHEN 0 THEN '(763) 323-6515'
        WHEN 1 THEN '(651) 603-7800'
        WHEN 2 THEN '(612) 879-7624'
        WHEN 3 THEN '(952) 442-4390'
    END,
    'Coordinator',
    0
FROM posts p
WHERE p.post_type = 'exchange'
  AND p.status = 'active'
  AND p.deleted_at IS NULL
  AND NOT EXISTS (SELECT 1 FROM contacts c WHERE c.contactable_type = 'post' AND c.contactable_id = p.id)
ON CONFLICT DO NOTHING;

INSERT INTO contacts (contactable_type, contactable_id, contact_type, contact_value, contact_label, display_order)
SELECT 'post', p.id, 'email',
    CASE (ROW_NUMBER() OVER (ORDER BY p.id)) % 4
        WHEN 0 THEN 'volunteer@communityaction.org'
        WHEN 1 THEN 'help@sharetheload.org'
        WHEN 2 THEN 'info@neighborexchange.org'
        WHEN 3 THEN 'connect@mnmutualaid.org'
    END,
    'Email',
    1
FROM posts p
WHERE p.post_type = 'exchange'
  AND p.status = 'active'
  AND p.deleted_at IS NULL
  AND NOT EXISTS (SELECT 1 FROM contacts c WHERE c.contactable_type = 'post' AND c.contactable_id = p.id AND c.contact_type = 'email')
ON CONFLICT DO NOTHING;

-- Notices: phone contact
INSERT INTO contacts (contactable_type, contactable_id, contact_type, contact_value, contact_label, display_order)
SELECT 'post', p.id, 'phone',
    CASE (ROW_NUMBER() OVER (ORDER BY p.id)) % 5
        WHEN 0 THEN '(763) 422-7075'
        WHEN 1 THEN '(651) 266-8989'
        WHEN 2 THEN '(612) 596-1253'
        WHEN 3 THEN '(952) 496-8686'
        WHEN 4 THEN '(763) 421-4760'
    END,
    'Information Line',
    0
FROM posts p
WHERE p.post_type = 'notice'
  AND p.status = 'active'
  AND p.deleted_at IS NULL
  AND NOT EXISTS (SELECT 1 FROM contacts c WHERE c.contactable_type = 'post' AND c.contactable_id = p.id)
ON CONFLICT DO NOTHING;

-- Spotlights: phone + website
INSERT INTO contacts (contactable_type, contactable_id, contact_type, contact_value, contact_label, display_order)
SELECT 'post', p.id, 'phone',
    CASE (ROW_NUMBER() OVER (ORDER BY p.id)) % 4
        WHEN 0 THEN '(763) 427-4430'
        WHEN 1 THEN '(651) 224-1385'
        WHEN 2 THEN '(612) 728-5767'
        WHEN 3 THEN '(952) 985-5300'
    END,
    'Main Office',
    0
FROM posts p
WHERE p.post_type = 'spotlight'
  AND p.status = 'active'
  AND p.deleted_at IS NULL
  AND NOT EXISTS (SELECT 1 FROM contacts c WHERE c.contactable_type = 'post' AND c.contactable_id = p.id)
ON CONFLICT DO NOTHING;

INSERT INTO contacts (contactable_type, contactable_id, contact_type, contact_value, contact_label, display_order)
SELECT 'post', p.id, 'website',
    CASE (ROW_NUMBER() OVER (ORDER BY p.id)) % 4
        WHEN 0 THEN 'https://www.anokacounty.us/services'
        WHEN 1 THEN 'https://www.communityactionmn.org'
        WHEN 2 THEN 'https://www.unitedwaytwincities.org'
        WHEN 3 THEN 'https://www.mnfoodshelf.org'
    END,
    'Website',
    1
FROM posts p
WHERE p.post_type = 'spotlight'
  AND p.status = 'active'
  AND p.deleted_at IS NULL
  AND NOT EXISTS (SELECT 1 FROM contacts c WHERE c.contactable_type = 'post' AND c.contactable_id = p.id AND c.contact_type = 'website')
ON CONFLICT DO NOTHING;

-- References: phone + website
INSERT INTO contacts (contactable_type, contactable_id, contact_type, contact_value, contact_label, display_order)
SELECT 'post', p.id, 'phone',
    CASE (ROW_NUMBER() OVER (ORDER BY p.id)) % 4
        WHEN 0 THEN '(763) 324-4000'
        WHEN 1 THEN '(651) 291-0211'
        WHEN 2 THEN '(612) 348-3000'
        WHEN 3 THEN '(952) 891-7500'
    END,
    'Helpline',
    0
FROM posts p
WHERE p.post_type = 'reference'
  AND p.status = 'active'
  AND p.deleted_at IS NULL
  AND NOT EXISTS (SELECT 1 FROM contacts c WHERE c.contactable_type = 'post' AND c.contactable_id = p.id)
ON CONFLICT DO NOTHING;

INSERT INTO contacts (contactable_type, contactable_id, contact_type, contact_value, contact_label, display_order)
SELECT 'post', p.id, 'website',
    CASE (ROW_NUMBER() OVER (ORDER BY p.id)) % 3
        WHEN 0 THEN 'https://www.211unitedway.org'
        WHEN 1 THEN 'https://mn.gov/dhs/people-we-serve'
        WHEN 2 THEN 'https://www.legalaidmn.org'
    END,
    'Online Resources',
    1
FROM posts p
WHERE p.post_type = 'reference'
  AND p.status = 'active'
  AND p.deleted_at IS NULL
  AND NOT EXISTS (SELECT 1 FROM contacts c WHERE c.contactable_type = 'post' AND c.contactable_id = p.id AND c.contact_type = 'website')
ON CONFLICT DO NOTHING;

-- Stories: source URL + email for tips
UPDATE posts SET source_url = CASE (HASHTEXT(id::text) % 5 + 5) % 5
    WHEN 0 THEN 'https://www.mprnews.org'
    WHEN 1 THEN 'https://www.startribune.com'
    WHEN 2 THEN 'https://www.minnpost.com'
    WHEN 3 THEN 'https://www.sahan.com'
    WHEN 4 THEN 'https://www.twincities.com'
END
WHERE post_type = 'story'
  AND status = 'active'
  AND deleted_at IS NULL
  AND source_url IS NULL;


-- ============================================================================
-- 2. POST_DATETIME — event dates for events that don't have them
-- ============================================================================

INSERT INTO post_datetime (post_id, start_at, end_at, cost, recurring)
SELECT p.id,
    -- Scatter events across the next 30 days
    NOW() + (((ROW_NUMBER() OVER (ORDER BY p.id)) % 30) || ' days')::interval
        + '10:00:00'::interval,
    NOW() + (((ROW_NUMBER() OVER (ORDER BY p.id)) % 30) || ' days')::interval
        + '14:00:00'::interval,
    CASE (ROW_NUMBER() OVER (ORDER BY p.id)) % 4
        WHEN 0 THEN 'Free'
        WHEN 1 THEN 'Free'
        WHEN 2 THEN '$5 suggested donation'
        WHEN 3 THEN 'Free, registration required'
    END,
    CASE WHEN (ROW_NUMBER() OVER (ORDER BY p.id)) % 5 = 0 THEN true ELSE false END
FROM posts p
WHERE p.post_type = 'event'
  AND p.status = 'active'
  AND p.deleted_at IS NULL
  AND NOT EXISTS (SELECT 1 FROM post_datetime pd WHERE pd.post_id = p.id)
ON CONFLICT (post_id) DO NOTHING;


-- ============================================================================
-- 3. POST_LINK — CTA links for notices that don't have them
-- ============================================================================

INSERT INTO post_link (post_id, url, label, deadline)
SELECT p.id,
    CASE (ROW_NUMBER() OVER (ORDER BY p.id)) % 5
        WHEN 0 THEN 'https://www.anokacounty.us/apply'
        WHEN 1 THEN 'https://mn.gov/dhs/apply-online'
        WHEN 2 THEN 'https://www.benefits.gov/benefit/1640'
        WHEN 3 THEN 'https://edocs.dhs.state.mn.us/lfserver/public/DHS-5223-ENG'
        WHEN 4 THEN 'https://www.healthcare.gov/get-coverage'
    END,
    CASE (ROW_NUMBER() OVER (ORDER BY p.id)) % 5
        WHEN 0 THEN 'Apply Online'
        WHEN 1 THEN 'Start Application'
        WHEN 2 THEN 'Check Eligibility'
        WHEN 3 THEN 'Download Form'
        WHEN 4 THEN 'Get Started'
    END,
    (NOW() + (((ROW_NUMBER() OVER (ORDER BY p.id)) % 60 + 7) || ' days')::interval)::date
FROM posts p
WHERE p.post_type = 'notice'
  AND p.status = 'active'
  AND p.deleted_at IS NULL
  AND NOT EXISTS (SELECT 1 FROM post_link pl WHERE pl.post_id = p.id)
ON CONFLICT (post_id) DO NOTHING;

-- Also add links for events (registration links)
INSERT INTO post_link (post_id, url, label)
SELECT p.id,
    CASE (ROW_NUMBER() OVER (ORDER BY p.id)) % 4
        WHEN 0 THEN 'https://www.eventbrite.com/e/community-event'
        WHEN 1 THEN 'https://www.signupgenius.com/go/community-signup'
        WHEN 2 THEN 'https://forms.gle/community-rsvp'
        WHEN 3 THEN 'https://www.anokacounty.us/events/register'
    END,
    CASE (ROW_NUMBER() OVER (ORDER BY p.id)) % 4
        WHEN 0 THEN 'Register Free'
        WHEN 1 THEN 'Sign Up'
        WHEN 2 THEN 'RSVP'
        WHEN 3 THEN 'Reserve Your Spot'
    END
FROM posts p
WHERE p.post_type = 'event'
  AND p.status = 'active'
  AND p.deleted_at IS NULL
  AND NOT EXISTS (SELECT 1 FROM post_link pl WHERE pl.post_id = p.id)
ON CONFLICT (post_id) DO NOTHING;


-- ============================================================================
-- 4. POST_SCHEDULE — hours for exchanges, spotlights, references
-- ============================================================================

-- Exchanges: weekday hours
INSERT INTO post_schedule (post_id, day, opens, closes, sort_order)
SELECT p.id, d.day, d.opens, d.closes, d.sort_order
FROM posts p
CROSS JOIN (VALUES
    ('Monday',    '09:00 AM', '04:00 PM', 0),
    ('Tuesday',   '09:00 AM', '04:00 PM', 1),
    ('Wednesday', '09:00 AM', '04:00 PM', 2),
    ('Thursday',  '09:00 AM', '04:00 PM', 3),
    ('Friday',    '09:00 AM', '02:00 PM', 4)
) AS d(day, opens, closes, sort_order)
WHERE p.post_type = 'exchange'
  AND p.status = 'active'
  AND p.deleted_at IS NULL
  AND NOT EXISTS (SELECT 1 FROM post_schedule ps WHERE ps.post_id = p.id)
ON CONFLICT DO NOTHING;

-- Spotlights: varied hours (skip if already has schedule)
INSERT INTO post_schedule (post_id, day, opens, closes, sort_order)
SELECT p.id, d.day, d.opens, d.closes, d.sort_order
FROM posts p
CROSS JOIN (VALUES
    ('Monday',    '8:00 AM',  '6:00 PM', 0),
    ('Tuesday',   '8:00 AM',  '6:00 PM', 1),
    ('Wednesday', '8:00 AM',  '8:00 PM', 2),
    ('Thursday',  '8:00 AM',  '6:00 PM', 3),
    ('Friday',    '8:00 AM',  '5:00 PM', 4),
    ('Saturday',  '10:00 AM', '2:00 PM', 5)
) AS d(day, opens, closes, sort_order)
WHERE p.post_type = 'spotlight'
  AND p.status = 'active'
  AND p.deleted_at IS NULL
  AND NOT EXISTS (SELECT 1 FROM post_schedule ps WHERE ps.post_id = p.id)
ON CONFLICT DO NOTHING;

-- References: M-F business hours
INSERT INTO post_schedule (post_id, day, opens, closes, sort_order)
SELECT p.id, d.day, d.opens, d.closes, d.sort_order
FROM posts p
CROSS JOIN (VALUES
    ('Monday',    '8:00 AM', '4:30 PM', 0),
    ('Tuesday',   '8:00 AM', '4:30 PM', 1),
    ('Wednesday', '8:00 AM', '4:30 PM', 2),
    ('Thursday',  '8:00 AM', '4:30 PM', 3),
    ('Friday',    '8:00 AM', '4:30 PM', 4)
) AS d(day, opens, closes, sort_order)
WHERE p.post_type = 'reference'
  AND p.status = 'active'
  AND p.deleted_at IS NULL
  AND NOT EXISTS (SELECT 1 FROM post_schedule ps WHERE ps.post_id = p.id)
ON CONFLICT DO NOTHING;


-- ============================================================================
-- 5. POST_STATUS — state for exchanges
-- ============================================================================

INSERT INTO post_status (post_id, state, verified)
SELECT p.id,
    CASE (ROW_NUMBER() OVER (ORDER BY p.id)) % 3
        WHEN 0 THEN 'needed'
        WHEN 1 THEN 'available'
        WHEN 2 THEN 'needed'
    END,
    CASE (ROW_NUMBER() OVER (ORDER BY p.id)) % 4
        WHEN 0 THEN 'verified'
        WHEN 1 THEN 'verified'
        WHEN 2 THEN 'unverified'
        WHEN 3 THEN 'verified'
    END
FROM posts p
WHERE p.post_type = 'exchange'
  AND p.status = 'active'
  AND p.deleted_at IS NULL
  AND NOT EXISTS (SELECT 1 FROM post_status ps WHERE ps.post_id = p.id)
ON CONFLICT (post_id) DO NOTHING;


-- ============================================================================
-- 6. POST_MEDIA — hero images for stories and events that lack them
-- ============================================================================

INSERT INTO post_media (post_id, image_url, caption, credit, sort_order)
SELECT p.id,
    CASE (ROW_NUMBER() OVER (ORDER BY p.id)) % 8
        WHEN 0 THEN 'https://placehold.co/800x500/15803d/ffffff?text=Community'
        WHEN 1 THEN 'https://placehold.co/800x500/0284c7/ffffff?text=Minnesota'
        WHEN 2 THEN 'https://placehold.co/800x500/7c3aed/ffffff?text=Together'
        WHEN 3 THEN 'https://placehold.co/800x500/dc2626/ffffff?text=Local+News'
        WHEN 4 THEN 'https://placehold.co/800x500/d97706/ffffff?text=Neighbors'
        WHEN 5 THEN 'https://placehold.co/800x500/059669/ffffff?text=Resources'
        WHEN 6 THEN 'https://placehold.co/800x500/2563eb/ffffff?text=Services'
        WHEN 7 THEN 'https://placehold.co/800x500/9333ea/ffffff?text=Events'
    END,
    '',
    'Minnesota, Together',
    0
FROM posts p
WHERE p.post_type IN ('story', 'event', 'spotlight')
  AND p.status = 'active'
  AND p.deleted_at IS NULL
  AND NOT EXISTS (SELECT 1 FROM post_media pm WHERE pm.post_id = p.id)
ON CONFLICT DO NOTHING;


-- ============================================================================
-- 7. POST_META — backfill byline + deck for posts missing them
-- ============================================================================

UPDATE post_meta SET
    byline = CASE WHEN byline IS NULL THEN 'Root Editorial Staff' ELSE byline END,
    deck = CASE WHEN deck IS NULL THEN NULL ELSE deck END
WHERE byline IS NULL;

-- Add post_meta for posts that have none at all
INSERT INTO post_meta (post_id, kicker, byline)
SELECT p.id,
    CASE p.post_type
        WHEN 'story'     THEN COALESCE(INITCAP(p.category), 'News')
        WHEN 'notice'    THEN 'Public Notice'
        WHEN 'exchange'  THEN 'Community Exchange'
        WHEN 'event'     THEN 'Upcoming Event'
        WHEN 'spotlight' THEN 'Spotlight'
        WHEN 'reference' THEN 'Resource Guide'
        ELSE COALESCE(INITCAP(p.category), 'Community')
    END,
    'Root Editorial Staff'
FROM posts p
WHERE p.status = 'active'
  AND p.deleted_at IS NULL
  AND NOT EXISTS (SELECT 1 FROM post_meta pm WHERE pm.post_id = p.id)
ON CONFLICT (post_id) DO NOTHING;


-- ============================================================================
-- 8. LOCATION — fill in location for posts that have none
-- ============================================================================

UPDATE posts SET location = CASE (HASHTEXT(id::text) % 8 + 8) % 8
    WHEN 0 THEN 'Anoka, MN'
    WHEN 1 THEN 'Coon Rapids, MN'
    WHEN 2 THEN 'Blaine, MN'
    WHEN 3 THEN 'Minneapolis, MN'
    WHEN 4 THEN 'St. Paul, MN'
    WHEN 5 THEN 'Brooklyn Park, MN'
    WHEN 6 THEN 'Fridley, MN'
    WHEN 7 THEN 'Columbia Heights, MN'
END
WHERE status = 'active'
  AND deleted_at IS NULL
  AND location IS NULL;

COMMIT;
