-- =============================================================================
-- Seed field group data for existing posts
-- =============================================================================
-- Populates post_media, post_person, post_items, post_meta, post_link,
-- post_source_attribution, post_datetime, post_status, and post_schedule
-- for the ~35 posts seeded in 000185_seed_diverse_posts.sql.
--
-- This exercises the full visual range of broadsheet post components.
-- =============================================================================

-- =============================================================================
-- STORY field groups — media + meta (kicker, byline, deck)
-- =============================================================================

-- Story 1: Affordable Housing
INSERT INTO post_media (id, post_id, image_url, caption, credit, sort_order) VALUES
('c0000001-0000-0000-0000-000000000001'::uuid, 'b0000001-0000-0000-0000-000000000001'::uuid,
 'https://placehold.co/800x500/2563eb/ffffff?text=Housing+Complex',
 'The new 48-unit affordable housing complex on Main Street in downtown Anoka.',
 'Anoka County Housing Authority', 0)
ON CONFLICT DO NOTHING;

INSERT INTO post_meta (id, post_id, kicker, byline, timestamp, deck) VALUES
('d0000001-0000-0000-0000-000000000001'::uuid, 'b0000001-0000-0000-0000-000000000001'::uuid,
 'Housing', 'Anoka County News', NOW() - INTERVAL '2 days',
 'A 48-unit development brings affordable one and two-bedroom apartments to downtown Anoka with on-site childcare and community gardens.')
ON CONFLICT DO NOTHING;

-- Story 2: Mobile Food Shelf
INSERT INTO post_media (id, post_id, image_url, caption, credit, sort_order) VALUES
('c0000001-0000-0000-0000-000000000002'::uuid, 'b0000001-0000-0000-0000-000000000002'::uuid,
 'https://placehold.co/800x500/15803d/ffffff?text=Mobile+Food+Shelf',
 'The refrigerated truck makes stops in Ham Lake, East Bethel, and Nowthen.',
 'Anoka County Mobile Food Shelf', 0)
ON CONFLICT DO NOTHING;

INSERT INTO post_meta (id, post_id, kicker, byline, timestamp, deck) VALUES
('d0000001-0000-0000-0000-000000000002'::uuid, 'b0000001-0000-0000-0000-000000000002'::uuid,
 'Food Access', 'Root Editorial Staff', NOW() - INTERVAL '1 day',
 'Three new weekly stops bring fresh produce, dairy, and pantry staples to rural communities previously underserved by food assistance programs.')
ON CONFLICT DO NOTHING;

-- Story 3: Mental Health Crisis Center
INSERT INTO post_meta (id, post_id, kicker, byline, timestamp, deck) VALUES
('d0000001-0000-0000-0000-000000000003'::uuid, 'b0000001-0000-0000-0000-000000000003'::uuid,
 'Healthcare', 'County Board Reporter', NOW() - INTERVAL '3 days',
 'The $2.3 million facility in Coon Rapids will offer walk-in crisis services, observation beds, and peer support.')
ON CONFLICT DO NOTHING;

-- Story 4: Spring Flooding
INSERT INTO post_media (id, post_id, image_url, caption, credit, sort_order) VALUES
('c0000001-0000-0000-0000-000000000003'::uuid, 'b0000001-0000-0000-0000-000000000004'::uuid,
 'https://placehold.co/800x500/dc2626/ffffff?text=Flood+Prep',
 'Sandbag stations will open at three locations along the Rum River.',
 'MN Emergency Management', 0)
ON CONFLICT DO NOTHING;

INSERT INTO post_meta (id, post_id, kicker, byline, timestamp, deck) VALUES
('d0000001-0000-0000-0000-000000000004'::uuid, 'b0000001-0000-0000-0000-000000000004'::uuid,
 'Emergency', 'Emergency Management Desk', NOW(),
 'Above-average snowpack means above-average spring runoff. Residents in flood-prone areas should prepare now.')
ON CONFLICT DO NOTHING;

-- Story 5: Digital Literacy
INSERT INTO post_meta (id, post_id, kicker, byline, timestamp, deck) VALUES
('d0000001-0000-0000-0000-000000000005'::uuid, 'b0000001-0000-0000-0000-000000000005'::uuid,
 'Education', 'Library System', NOW() - INTERVAL '5 days',
 'Free classes in smartphone basics, online safety, telehealth, and job search — available in English, Spanish, and Somali.')
ON CONFLICT DO NOTHING;

-- =============================================================================
-- NOTICE field groups — source attribution + link (for action notices)
-- =============================================================================

-- Notice 1: Heating Assistance (action notice with deadline)
INSERT INTO post_source_attribution (id, post_id, source_name, attribution) VALUES
('e0000001-0000-0000-0000-000000000001'::uuid, 'b0000002-0000-0000-0000-000000000001'::uuid,
 'Anoka County Human Services', 'Energy Assistance Program')
ON CONFLICT DO NOTHING;

INSERT INTO post_link (id, post_id, label, url, deadline) VALUES
('f0000001-0000-0000-0000-000000000001'::uuid, 'b0000002-0000-0000-0000-000000000001'::uuid,
 'Apply Now', 'https://www.anokacounty.us/energy-assistance', '2026-03-31')
ON CONFLICT DO NOTHING;

-- Notice 2: Free Tax Prep
INSERT INTO post_source_attribution (id, post_id, source_name, attribution) VALUES
('e0000001-0000-0000-0000-000000000002'::uuid, 'b0000002-0000-0000-0000-000000000002'::uuid,
 'VITA Program', 'IRS Volunteer Income Tax Assistance')
ON CONFLICT DO NOTHING;

-- Notice 3: WIC Extended Hours
INSERT INTO post_source_attribution (id, post_id, source_name, attribution) VALUES
('e0000001-0000-0000-0000-000000000003'::uuid, 'b0000002-0000-0000-0000-000000000003'::uuid,
 'Anoka County WIC', 'Women, Infants, and Children Program')
ON CONFLICT DO NOTHING;

-- Notice 4: Road Construction
INSERT INTO post_source_attribution (id, post_id, source_name, attribution) VALUES
('e0000001-0000-0000-0000-000000000004'::uuid, 'b0000002-0000-0000-0000-000000000004'::uuid,
 'MnDOT', 'Minnesota Department of Transportation')
ON CONFLICT DO NOTHING;

-- Notice 5: Tornado Sirens
INSERT INTO post_source_attribution (id, post_id, source_name, attribution) VALUES
('e0000001-0000-0000-0000-000000000005'::uuid, 'b0000002-0000-0000-0000-000000000005'::uuid,
 'Anoka County Emergency Management', NULL)
ON CONFLICT DO NOTHING;

-- Notice 10: Summer Youth Employment (action notice)
INSERT INTO post_source_attribution (id, post_id, source_name, attribution) VALUES
('e0000001-0000-0000-0000-000000000010'::uuid, 'b0000002-0000-0000-0000-000000000010'::uuid,
 'Anoka County Workforce Center', NULL)
ON CONFLICT DO NOTHING;

INSERT INTO post_link (id, post_id, label, url, deadline) VALUES
('f0000001-0000-0000-0000-000000000002'::uuid, 'b0000002-0000-0000-0000-000000000010'::uuid,
 'Apply Now', 'https://www.anokacounty.us/youth-employment', '2026-05-01')
ON CONFLICT DO NOTHING;

-- =============================================================================
-- EXCHANGE field groups — items + status
-- =============================================================================

-- Exchange 1: Volunteer Drivers (need)
INSERT INTO post_items (id, post_id, name, detail, sort_order) VALUES
('a1000001-0000-0000-0000-000000000001'::uuid, 'b0000003-0000-0000-0000-000000000001'::uuid,
 'Drivers needed', 'Monday through Friday, 11 AM–1 PM', 0),
('a1000001-0000-0000-0000-000000000002'::uuid, 'b0000003-0000-0000-0000-000000000001'::uuid,
 'Mileage reimbursement', 'Provided for all routes', 1),
('a1000001-0000-0000-0000-000000000003'::uuid, 'b0000003-0000-0000-0000-000000000001'::uuid,
 'Background check', 'Required for all volunteers', 2)
ON CONFLICT DO NOTHING;

INSERT INTO post_status (id, post_id, state, verified) VALUES
('a2000001-0000-0000-0000-000000000001'::uuid, 'b0000003-0000-0000-0000-000000000001'::uuid,
 'needed', '2026-03-15')
ON CONFLICT DO NOTHING;

-- Exchange 2: Winter Coats (aid)
INSERT INTO post_items (id, post_id, name, detail, sort_order) VALUES
('a1000002-0000-0000-0000-000000000001'::uuid, 'b0000003-0000-0000-0000-000000000002'::uuid,
 'Adult winter coats', 'Sizes M through 3XL', 0),
('a1000002-0000-0000-0000-000000000002'::uuid, 'b0000003-0000-0000-0000-000000000002'::uuid,
 'Pickup location', 'Community Action Center', 1)
ON CONFLICT DO NOTHING;

INSERT INTO post_status (id, post_id, state, verified) VALUES
('a2000002-0000-0000-0000-000000000001'::uuid, 'b0000003-0000-0000-0000-000000000002'::uuid,
 'available', '2026-03-14')
ON CONFLICT DO NOTHING;

-- Exchange 3: Bicycle Repair (aid)
INSERT INTO post_status (id, post_id, state, verified) VALUES
('a2000003-0000-0000-0000-000000000001'::uuid, 'b0000003-0000-0000-0000-000000000003'::uuid,
 'available', '2026-03-12')
ON CONFLICT DO NOTHING;

-- Exchange 4: Spanish Tutors (need)
INSERT INTO post_items (id, post_id, name, detail, sort_order) VALUES
('a1000004-0000-0000-0000-000000000001'::uuid, 'b0000003-0000-0000-0000-000000000004'::uuid,
 'Bilingual tutors', 'Spanish–English speakers', 0),
('a1000004-0000-0000-0000-000000000002'::uuid, 'b0000003-0000-0000-0000-000000000004'::uuid,
 'Time commitment', '2 hours per week', 1),
('a1000004-0000-0000-0000-000000000003'::uuid, 'b0000003-0000-0000-0000-000000000004'::uuid,
 'Training', 'Provided by district', 2)
ON CONFLICT DO NOTHING;

INSERT INTO post_status (id, post_id, state, verified) VALUES
('a2000004-0000-0000-0000-000000000001'::uuid, 'b0000003-0000-0000-0000-000000000004'::uuid,
 'needed', '2026-03-10')
ON CONFLICT DO NOTHING;

-- Exchange 5: Tool Library Donations (need)
INSERT INTO post_items (id, post_id, name, detail, sort_order) VALUES
('a1000005-0000-0000-0000-000000000001'::uuid, 'b0000003-0000-0000-0000-000000000005'::uuid,
 'Hand tools', 'Hammers, screwdrivers, wrenches', 0),
('a1000005-0000-0000-0000-000000000002'::uuid, 'b0000003-0000-0000-0000-000000000005'::uuid,
 'Power tools', 'Drills, saws, sanders', 1),
('a1000005-0000-0000-0000-000000000003'::uuid, 'b0000003-0000-0000-0000-000000000005'::uuid,
 'Garden equipment', 'Shovels, rakes, hoses', 2)
ON CONFLICT DO NOTHING;

INSERT INTO post_status (id, post_id, state, verified) VALUES
('a2000005-0000-0000-0000-000000000001'::uuid, 'b0000003-0000-0000-0000-000000000005'::uuid,
 'needed', '2026-03-11')
ON CONFLICT DO NOTHING;

-- =============================================================================
-- EVENT field groups — datetime
-- =============================================================================

-- Event 1: Community Resource Fair — March 22, 10am–3pm, Free
INSERT INTO post_datetime (id, post_id, start_at, end_at, cost, recurring) VALUES
('a3000001-0000-0000-0000-000000000001'::uuid, 'b0000004-0000-0000-0000-000000000001'::uuid,
 '2026-03-22 10:00:00-05', '2026-03-22 15:00:00-05', 'Free', false)
ON CONFLICT DO NOTHING;

-- Event 2: Narcan Training — March 20, 6–8pm, Free
INSERT INTO post_datetime (id, post_id, start_at, end_at, cost, recurring) VALUES
('a3000002-0000-0000-0000-000000000001'::uuid, 'b0000004-0000-0000-0000-000000000002'::uuid,
 '2026-03-20 18:00:00-05', '2026-03-20 20:00:00-05', 'Free', false)
ON CONFLICT DO NOTHING;

-- Event 3: Renters Rights — March 27, 5:30–7:30pm, Free
INSERT INTO post_datetime (id, post_id, start_at, end_at, cost, recurring) VALUES
('a3000003-0000-0000-0000-000000000001'::uuid, 'b0000004-0000-0000-0000-000000000003'::uuid,
 '2026-03-27 17:30:00-05', '2026-03-27 19:30:00-05', 'Free', false)
ON CONFLICT DO NOTHING;

-- Event 4: Spring Job Fair — April 3, 10am–2pm, Free
INSERT INTO post_datetime (id, post_id, start_at, end_at, cost, recurring) VALUES
('a3000004-0000-0000-0000-000000000001'::uuid, 'b0000004-0000-0000-0000-000000000004'::uuid,
 '2026-04-03 10:00:00-05', '2026-04-03 14:00:00-05', 'Free', false)
ON CONFLICT DO NOTHING;

-- Event 5: Family Storytime — recurring Wednesdays at 10:30am
INSERT INTO post_datetime (id, post_id, start_at, end_at, cost, recurring) VALUES
('a3000005-0000-0000-0000-000000000001'::uuid, 'b0000004-0000-0000-0000-000000000005'::uuid,
 '2026-03-19 10:30:00-05', '2026-03-19 11:30:00-05', 'Free', true)
ON CONFLICT DO NOTHING;

-- =============================================================================
-- SPOTLIGHT field groups — person + media
-- =============================================================================

-- Spotlight 1: Alexandra House (business spotlight)
INSERT INTO post_meta (id, post_id, kicker, byline, timestamp) VALUES
('d0000005-0000-0000-0000-000000000001'::uuid, 'b0000005-0000-0000-0000-000000000001'::uuid,
 'Community Organization', 'Root Editorial Staff', NOW() - INTERVAL '4 days')
ON CONFLICT DO NOTHING;

-- Spotlight 2: Fatima Hassan (person spotlight)
INSERT INTO post_person (id, post_id, name, role, bio, photo_url, quote) VALUES
('a4000001-0000-0000-0000-000000000001'::uuid, 'b0000005-0000-0000-0000-000000000002'::uuid,
 'Fatima Hassan', 'Community Garden Coordinator',
 'Fatima has been coordinating the Coon Rapids Community Garden for three years, helping 60 families grow their own food.',
 'https://placehold.co/200x200/8b5cf6/ffffff?text=FH',
 'When people grow food together, they grow community together.')
ON CONFLICT DO NOTHING;

INSERT INTO post_media (id, post_id, image_url, caption, credit, sort_order) VALUES
('c0000005-0000-0000-0000-000000000001'::uuid, 'b0000005-0000-0000-0000-000000000002'::uuid,
 'https://placehold.co/600x400/15803d/ffffff?text=Community+Garden',
 'The Coon Rapids Community Garden serves 60 families.',
 'Root Editorial', 0)
ON CONFLICT DO NOTHING;

-- Spotlight 3: Dave Peterson (person spotlight)
INSERT INTO post_person (id, post_id, name, role, bio, photo_url, quote) VALUES
('a4000002-0000-0000-0000-000000000001'::uuid, 'b0000005-0000-0000-0000-000000000003'::uuid,
 'Dave Peterson', 'Retired Teacher & Tutor',
 'Former Anoka High School math teacher who now offers free after-school tutoring three days a week.',
 'https://placehold.co/200x200/2563eb/ffffff?text=DP',
 'Every kid deserves someone who believes they can do the math.')
ON CONFLICT DO NOTHING;

-- =============================================================================
-- REFERENCE field groups — items + meta (updated)
-- =============================================================================

-- Reference 1: Food Shelves Directory
INSERT INTO post_items (id, post_id, name, detail, sort_order) VALUES
('a1000006-0000-0000-0000-000000000001'::uuid, 'b0000006-0000-0000-0000-000000000001'::uuid,
 'ACBC Food Shelf', 'Mon/Wed 9am–3pm, Fri 9am–12pm', 0),
('a1000006-0000-0000-0000-000000000002'::uuid, 'b0000006-0000-0000-0000-000000000001'::uuid,
 'Mercy Place Food Shelf', 'Tue/Thu 10am–2pm', 1),
('a1000006-0000-0000-0000-000000000003'::uuid, 'b0000006-0000-0000-0000-000000000001'::uuid,
 'CROSS Services', 'Wed 12pm–4pm, Sat 9am–12pm', 2),
('a1000006-0000-0000-0000-000000000004'::uuid, 'b0000006-0000-0000-0000-000000000001'::uuid,
 'Anoka County Mobile Food Shelf', 'See schedule for routes', 3),
('a1000006-0000-0000-0000-000000000005'::uuid, 'b0000006-0000-0000-0000-000000000001'::uuid,
 'SNAP Enrollment', 'County Human Services, M–F 8am–4:30pm', 4)
ON CONFLICT DO NOTHING;

INSERT INTO post_meta (id, post_id, kicker, updated) VALUES
('d0000006-0000-0000-0000-000000000001'::uuid, 'b0000006-0000-0000-0000-000000000001'::uuid,
 'Resource Directory', 'Updated March 2026')
ON CONFLICT DO NOTHING;

-- Reference 2: Emergency Services
INSERT INTO post_items (id, post_id, name, detail, sort_order) VALUES
('a1000007-0000-0000-0000-000000000001'::uuid, 'b0000006-0000-0000-0000-000000000002'::uuid,
 'Crisis Hotline', '988 (Suicide & Crisis Lifeline)', 0),
('a1000007-0000-0000-0000-000000000002'::uuid, 'b0000006-0000-0000-0000-000000000002'::uuid,
 'Warming Centers', 'Anoka County 763-324-1500', 1),
('a1000007-0000-0000-0000-000000000003'::uuid, 'b0000006-0000-0000-0000-000000000002'::uuid,
 'Emergency Shelter', 'Alexandra House 763-780-2330', 2),
('a1000007-0000-0000-0000-000000000004'::uuid, 'b0000006-0000-0000-0000-000000000002'::uuid,
 'Poison Control', '1-800-222-1222', 3),
('a1000007-0000-0000-0000-000000000005'::uuid, 'b0000006-0000-0000-0000-000000000002'::uuid,
 'Domestic Violence', 'Day One 866-223-1111', 4)
ON CONFLICT DO NOTHING;

INSERT INTO post_meta (id, post_id, kicker, updated) VALUES
('d0000006-0000-0000-0000-000000000002'::uuid, 'b0000006-0000-0000-0000-000000000002'::uuid,
 'Quick Reference', 'Updated February 2026')
ON CONFLICT DO NOTHING;

-- Reference 3: Legal Aid
INSERT INTO post_items (id, post_id, name, detail, sort_order) VALUES
('a1000008-0000-0000-0000-000000000001'::uuid, 'b0000006-0000-0000-0000-000000000003'::uuid,
 'Legal Aid Society', 'Family law, housing, benefits', 0),
('a1000008-0000-0000-0000-000000000002'::uuid, 'b0000006-0000-0000-0000-000000000003'::uuid,
 'Volunteer Lawyers Network', 'Free clinics monthly', 1),
('a1000008-0000-0000-0000-000000000003'::uuid, 'b0000006-0000-0000-0000-000000000003'::uuid,
 'Mid-Minnesota Legal Aid', 'Immigration, consumer rights', 2),
('a1000008-0000-0000-0000-000000000004'::uuid, 'b0000006-0000-0000-0000-000000000003'::uuid,
 'LawHelpMN.org', 'Self-help legal resources online', 3)
ON CONFLICT DO NOTHING;

INSERT INTO post_meta (id, post_id, kicker, updated) VALUES
('d0000006-0000-0000-0000-000000000003'::uuid, 'b0000006-0000-0000-0000-000000000003'::uuid,
 'Resource Directory', 'Updated January 2026')
ON CONFLICT DO NOTHING;

-- =============================================================================
-- SCHEDULE field groups — for a spotlight business and a reference
-- =============================================================================

-- Spotlight 1: Alexandra House schedule
INSERT INTO post_schedule (id, post_id, day, opens, closes, sort_order) VALUES
('a5000001-0000-0000-0000-000000000001'::uuid, 'b0000005-0000-0000-0000-000000000001'::uuid,
 'Monday', '8:00 AM', '8:00 PM', 0),
('a5000001-0000-0000-0000-000000000002'::uuid, 'b0000005-0000-0000-0000-000000000001'::uuid,
 'Tuesday', '8:00 AM', '8:00 PM', 1),
('a5000001-0000-0000-0000-000000000003'::uuid, 'b0000005-0000-0000-0000-000000000001'::uuid,
 'Wednesday', '8:00 AM', '8:00 PM', 2),
('a5000001-0000-0000-0000-000000000004'::uuid, 'b0000005-0000-0000-0000-000000000001'::uuid,
 'Thursday', '8:00 AM', '8:00 PM', 3),
('a5000001-0000-0000-0000-000000000005'::uuid, 'b0000005-0000-0000-0000-000000000001'::uuid,
 'Friday', '8:00 AM', '5:00 PM', 4),
('a5000001-0000-0000-0000-000000000006'::uuid, 'b0000005-0000-0000-0000-000000000001'::uuid,
 'Saturday', '24-hour crisis line', '24-hour crisis line', 5),
('a5000001-0000-0000-0000-000000000007'::uuid, 'b0000005-0000-0000-0000-000000000001'::uuid,
 'Sunday', '24-hour crisis line', '24-hour crisis line', 6)
ON CONFLICT DO NOTHING;
