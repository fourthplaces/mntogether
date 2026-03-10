-- =============================================================================
-- Seed diverse posts for Anoka County to test layout engine slot filling
-- =============================================================================
-- Creates ~35 posts with realistic titles/descriptions across all 6 post types
-- and all 3 weight tiers, plus location mappings to Anoka County zip codes.
-- =============================================================================

-- Use Anoka County zip code 55303 for location mapping
-- (maps to Anoka county via zip_counties proximity in 000174)

-- Create locations in Anoka County (reusable across posts)
INSERT INTO locations (id, name, city, state, postal_code, location_type, created_at, updated_at) VALUES
('a0000001-0000-0000-0000-000000000001'::uuid, 'Anoka County Community Center', 'Anoka', 'MN', '55303', 'physical', NOW(), NOW()),
('a0000001-0000-0000-0000-000000000002'::uuid, 'Coon Rapids Civic Center', 'Coon Rapids', 'MN', '55433', 'physical', NOW(), NOW()),
('a0000001-0000-0000-0000-000000000003'::uuid, 'Blaine Community Resource Hub', 'Blaine', 'MN', '55434', 'physical', NOW(), NOW()),
('a0000001-0000-0000-0000-000000000004'::uuid, 'Andover Family Services', 'Andover', 'MN', '55304', 'physical', NOW(), NOW()),
('a0000001-0000-0000-0000-000000000005'::uuid, 'Circle Pines Outreach', 'Circle Pines', 'MN', '55014', 'physical', NOW(), NOW())
ON CONFLICT DO NOTHING;

-- Ensure zip codes exist in zip_codes table (they should from 000116, but be safe)
INSERT INTO zip_codes (zip_code, city, state, latitude, longitude) VALUES
('55433', 'Coon Rapids', 'MN', 45.1200, -93.3030),
('55434', 'Blaine', 'MN', 45.1608, -93.2355),
('55304', 'Andover', 'MN', 45.2330, -93.2914),
('55014', 'Circle Pines', 'MN', 45.1486, -93.1514)
ON CONFLICT DO NOTHING;

-- Ensure zip_counties mappings exist for Anoka County
INSERT INTO zip_counties (zip_code, county_id, is_primary)
SELECT z.zip_code, c.id, true
FROM (VALUES ('55433'), ('55434'), ('55304'), ('55014')) AS z(zip_code)
CROSS JOIN counties c
WHERE c.fips_code = '27003'
ON CONFLICT DO NOTHING;

-- =============================================================================
-- STORY posts (heavy weight) — 5 posts
-- =============================================================================
INSERT INTO posts (id, title, description, post_type, category, weight, priority, status, source_language, created_at, updated_at) VALUES
('b0000001-0000-0000-0000-000000000001'::uuid,
 'Anoka County Opens New Affordable Housing Complex on Main Street',
 'A 48-unit affordable housing complex opened this week in downtown Anoka, providing one and two-bedroom apartments for families earning below 60% of area median income. The development includes on-site childcare, a community garden, and connections to county transit. Applications are now being accepted through Anoka County Housing Authority.',
 'story', 'housing', 'heavy', 90, 'active', 'en', NOW(), NOW()),

('b0000001-0000-0000-0000-000000000002'::uuid,
 'Mobile Food Shelf Expands Routes to Reach Rural Anoka County',
 'The Anoka County Mobile Food Shelf announced three new weekly stops in rural areas previously underserved by food assistance programs. Beginning next Monday, the refrigerated truck will visit Ham Lake, East Bethel, and Nowthen with fresh produce, dairy, and pantry staples. No income verification is required.',
 'story', 'food', 'heavy', 85, 'active', 'en', NOW(), NOW()),

('b0000001-0000-0000-0000-000000000003'::uuid,
 'County Board Approves Mental Health Crisis Center Funding',
 'The Anoka County Board of Commissioners voted unanimously to allocate $2.3 million for a new mental health crisis stabilization center in Coon Rapids. The facility will offer walk-in crisis services, 23-hour observation beds, and peer support specialists. Construction is expected to begin this fall.',
 'story', 'healthcare', 'heavy', 88, 'active', 'en', NOW(), NOW()),

('b0000001-0000-0000-0000-000000000004'::uuid,
 'Spring Flooding Preparations Underway Along Rum River Communities',
 'Emergency management officials are coordinating flood preparation efforts along the Rum River as snowpack levels suggest above-average spring runoff. Sandbag stations will be available at three locations starting next week. Residents in flood-prone areas should review their emergency plans and insurance coverage.',
 'story', 'other', 'heavy', 92, 'active', 'en', NOW(), NOW()),

('b0000001-0000-0000-0000-000000000005'::uuid,
 'Anoka County Library System Launches Digital Literacy Program',
 'All seven Anoka County library branches will offer free digital literacy classes starting in April. The program covers smartphone basics, online safety, telehealth navigation, and job search skills. Classes are available in English, Spanish, and Somali with free childcare provided.',
 'story', 'education', 'heavy', 80, 'active', 'en', NOW(), NOW());

-- =============================================================================
-- NOTICE posts (light weight) — 10 posts
-- =============================================================================
INSERT INTO posts (id, title, description, post_type, category, weight, priority, status, source_language, created_at, updated_at) VALUES
('b0000002-0000-0000-0000-000000000001'::uuid,
 'Heating Assistance Applications Due March 31',
 'The Energy Assistance Program deadline is approaching. Households earning below 50% SMI may qualify for help with heating bills. Apply at Anoka County Human Services or call 763-324-1500.',
 'notice', 'utilities', 'light', 75, 'active', 'en', NOW(), NOW()),

('b0000002-0000-0000-0000-000000000002'::uuid,
 'Free Tax Preparation Available at Anoka County Libraries',
 'VITA volunteers are offering free tax preparation for households earning under $64,000. Available Saturdays through April 15 at Anoka, Coon Rapids, and Blaine library branches. Bring photo ID, Social Security cards, and all tax documents.',
 'notice', 'financial', 'light', 70, 'active', 'en', NOW(), NOW()),

('b0000002-0000-0000-0000-000000000003'::uuid,
 'WIC Office Hours Extended Through Spring',
 'The Anoka County WIC office is now open Tuesdays and Thursdays until 7 PM to accommodate working families. Walk-ins welcome for benefits recertification.',
 'notice', 'food', 'light', 60, 'active', 'en', NOW(), NOW()),

('b0000002-0000-0000-0000-000000000004'::uuid,
 'Road Construction Alert: Highway 10 Detour Begins April 1',
 'MnDOT construction on Highway 10 between Anoka and Ramsey will require a detour starting April 1 through October. Metro Transit Route 852 will be rerouted via Main Street.',
 'notice', 'transportation', 'light', 65, 'active', 'en', NOW(), NOW()),

('b0000002-0000-0000-0000-000000000005'::uuid,
 'Severe Weather Awareness Week: Test Tornado Sirens Thursday',
 'Anoka County will test tornado sirens Thursday at 1:45 PM and 6:55 PM as part of Severe Weather Awareness Week. Know your shelter plan.',
 'notice', 'other', 'light', 55, 'active', 'en', NOW(), NOW()),

('b0000002-0000-0000-0000-000000000006'::uuid,
 'Medication Take-Back Event Saturday at Coon Rapids Civic Center',
 'Safely dispose of unused or expired medications. No questions asked. Accepted: prescription drugs, OTC medications, vitamins. Not accepted: needles, liquids, or inhalers.',
 'notice', 'healthcare', 'light', 50, 'active', 'en', NOW(), NOW()),

('b0000002-0000-0000-0000-000000000007'::uuid,
 'Pet Licensing Renewal Deadline Extended to April 15',
 'Anoka County Animal Control reminds pet owners that dog and cat licenses are due. License online at anokacounty.us/pets or at any county service center. Proof of rabies vaccination required.',
 'notice', 'other', 'light', 40, 'active', 'en', NOW(), NOW()),

('b0000002-0000-0000-0000-000000000008'::uuid,
 'Community Garden Plot Registration Opens March 15',
 'Register for a plot at one of four Anoka County community gardens. Plots are 10x20 feet, $25 per season. Water provided. First-come, first-served.',
 'notice', 'food', 'light', 45, 'active', 'en', NOW(), NOW()),

('b0000002-0000-0000-0000-000000000009'::uuid,
 'Anoka County Board Meeting Moved to March 25',
 'The regular Anoka County Board of Commissioners meeting originally scheduled for March 18 has been rescheduled to March 25 at 9 AM. Public comment period will be available.',
 'notice', 'other', 'light', 35, 'active', 'en', NOW(), NOW()),

('b0000002-0000-0000-0000-000000000010'::uuid,
 'Summer Youth Employment Program Applications Now Open',
 'Anoka County Workforce Center is accepting applications for the Summer Youth Employment Program. Ages 14-21. Earn minimum wage while gaining work experience. Apply by May 1.',
 'notice', 'employment', 'light', 58, 'active', 'en', NOW(), NOW());

-- =============================================================================
-- EXCHANGE posts (medium weight) — 5 posts
-- =============================================================================
INSERT INTO posts (id, title, description, post_type, category, weight, priority, status, source_language, created_at, updated_at) VALUES
('b0000003-0000-0000-0000-000000000001'::uuid,
 'Seeking Volunteer Drivers for Senior Meal Delivery',
 'Anoka County Senior Services needs volunteer drivers to deliver meals to homebound seniors. Routes available Monday through Friday, 11 AM to 1 PM. Mileage reimbursement provided. Background check required. Contact: 763-324-1600.',
 'exchange', 'other', 'medium', 72, 'active', 'en', NOW(), NOW()),

('b0000003-0000-0000-0000-000000000002'::uuid,
 'Free Gently Used Winter Coats Available',
 'Coats for Kids collection has surplus adult winter coats available at the Anoka County Community Action Center. Sizes M-3XL. First come, first served. Open weekdays 9 AM-4 PM.',
 'exchange', 'clothing', 'medium', 65, 'active', 'en', NOW(), NOW()),

('b0000003-0000-0000-0000-000000000003'::uuid,
 'Offering Free Bicycle Repair Workshop',
 'Anoka County Parks is hosting a free bicycle repair workshop at Bunker Hills. Learn basic maintenance, tire changing, and brake adjustment. Bring your bike. Tools provided. Saturday 10 AM-2 PM.',
 'exchange', 'transportation', 'medium', 55, 'active', 'en', NOW(), NOW()),

('b0000003-0000-0000-0000-000000000004'::uuid,
 'Looking for Spanish-Speaking Tutors for ESL Program',
 'The Anoka-Hennepin School District ESL program needs bilingual tutors to help adult learners. Two hours per week commitment. Training provided. Contact Maria at esl@anoka.k12.mn.us.',
 'exchange', 'education', 'medium', 60, 'active', 'en', NOW(), NOW()),

('b0000003-0000-0000-0000-000000000005'::uuid,
 'Tool Library Now Accepting Donations',
 'The Coon Rapids Tool Library accepts donations of hand tools, power tools, and gardening equipment. All donated items are available for free community borrowing. Drop off at the Civic Center Tuesdays 4-7 PM.',
 'exchange', 'other', 'medium', 50, 'active', 'en', NOW(), NOW());

-- =============================================================================
-- EVENT posts (medium weight) — 5 posts
-- =============================================================================
INSERT INTO posts (id, title, description, post_type, category, weight, priority, status, source_language, created_at, updated_at) VALUES
('b0000004-0000-0000-0000-000000000001'::uuid,
 'Community Resource Fair — Saturday March 22',
 'Over 40 local organizations will be at the Anoka County Fairgrounds providing information on housing, food assistance, healthcare, legal aid, and employment services. Free lunch, childcare, and interpreters available. 10 AM-3 PM.',
 'event', 'other', 'medium', 78, 'active', 'en', NOW(), NOW()),

('b0000004-0000-0000-0000-000000000002'::uuid,
 'Narcan Training and Distribution Event',
 'Free naloxone training and Narcan distribution at the Blaine Community Center. Learn how to recognize an opioid overdose and administer life-saving medication. No registration required. March 20, 6-8 PM.',
 'event', 'healthcare', 'medium', 70, 'active', 'en', NOW(), NOW()),

('b0000004-0000-0000-0000-000000000003'::uuid,
 'Renters Rights Workshop at Anoka County Law Library',
 'Free workshop on tenant rights in Minnesota. Topics include eviction protections, repair requests, security deposits, and lease agreements. Legal aid attorneys will answer questions. March 27, 5:30-7:30 PM.',
 'event', 'legal', 'medium', 68, 'active', 'en', NOW(), NOW()),

('b0000004-0000-0000-0000-000000000004'::uuid,
 'Spring Job Fair at Anoka Technical College',
 'Meet employers from healthcare, manufacturing, and skilled trades. On-site interviews available. Bring your resume. Professional attire recommended. April 3, 10 AM-2 PM.',
 'event', 'employment', 'medium', 62, 'active', 'en', NOW(), NOW()),

('b0000004-0000-0000-0000-000000000005'::uuid,
 'Family Storytime and Resource Connection at Ham Lake Library',
 'Weekly storytime for ages 0-5 with a twist: each week features a community resource partner. This week: Anoka County Early Childhood screening. Every Wednesday at 10:30 AM.',
 'event', 'education', 'medium', 48, 'active', 'en', NOW(), NOW());

-- =============================================================================
-- SPOTLIGHT posts (medium weight) — 3 posts
-- =============================================================================
INSERT INTO posts (id, title, description, post_type, category, weight, priority, status, source_language, created_at, updated_at) VALUES
('b0000005-0000-0000-0000-000000000001'::uuid,
 'Alexandra House: 40 Years Serving Domestic Violence Survivors',
 'This year marks 40 years of Alexandra House providing emergency shelter, legal advocacy, and support services to survivors of domestic violence in Anoka County. Their 24-hour crisis line has answered over 200,000 calls since opening.',
 'spotlight', 'other', 'medium', 73, 'active', 'en', NOW(), NOW()),

('b0000005-0000-0000-0000-000000000002'::uuid,
 'Meet Your Neighbor: Coon Rapids Community Garden Coordinator',
 'Fatima Hassan has been coordinating the Coon Rapids Community Garden for three years, helping 60 families grow their own food. She also runs cooking classes using garden-grown produce.',
 'spotlight', 'food', 'medium', 58, 'active', 'en', NOW(), NOW()),

('b0000005-0000-0000-0000-000000000003'::uuid,
 'Local Hero: Retired Teacher Runs Free Homework Help',
 'Former Anoka High School math teacher Dave Peterson offers free after-school tutoring three days a week at the Anoka Community Center. Over 50 students benefit weekly from his program.',
 'spotlight', 'education', 'medium', 52, 'active', 'en', NOW(), NOW());

-- =============================================================================
-- REFERENCE posts (medium weight) — 3 posts
-- =============================================================================
INSERT INTO posts (id, title, description, post_type, category, weight, priority, status, source_language, created_at, updated_at) VALUES
('b0000006-0000-0000-0000-000000000001'::uuid,
 'Anoka County Food Shelves Directory',
 'Complete list of food shelves in Anoka County with hours, eligibility, and contact information. Updated monthly. Includes SNAP enrollment assistance locations.',
 'reference', 'food', 'medium', 45, 'active', 'en', NOW(), NOW()),

('b0000006-0000-0000-0000-000000000002'::uuid,
 'Emergency Services Quick Reference',
 'Key phone numbers and locations for Anoka County emergency services: crisis hotline, warming centers, emergency shelter, poison control, and domestic violence support.',
 'reference', 'other', 'medium', 42, 'active', 'en', NOW(), NOW()),

('b0000006-0000-0000-0000-000000000003'::uuid,
 'Free Legal Aid Resources in Anoka County',
 'Directory of free and low-cost legal services available to Anoka County residents. Covers family law, housing, immigration, consumer rights, and public benefits.',
 'reference', 'legal', 'medium', 40, 'active', 'en', NOW(), NOW());

-- =============================================================================
-- Link posts to locations via locationables
-- =============================================================================
-- Distribute posts across the 5 Anoka County locations

-- Stories → Location 1 (Anoka Community Center) and Location 2 (Coon Rapids)
INSERT INTO locationables (id, location_id, locatable_type, locatable_id, is_primary, added_at) VALUES
(gen_random_uuid(), 'a0000001-0000-0000-0000-000000000001'::uuid, 'post', 'b0000001-0000-0000-0000-000000000001'::uuid, true, NOW()),
(gen_random_uuid(), 'a0000001-0000-0000-0000-000000000002'::uuid, 'post', 'b0000001-0000-0000-0000-000000000002'::uuid, true, NOW()),
(gen_random_uuid(), 'a0000001-0000-0000-0000-000000000002'::uuid, 'post', 'b0000001-0000-0000-0000-000000000003'::uuid, true, NOW()),
(gen_random_uuid(), 'a0000001-0000-0000-0000-000000000001'::uuid, 'post', 'b0000001-0000-0000-0000-000000000004'::uuid, true, NOW()),
(gen_random_uuid(), 'a0000001-0000-0000-0000-000000000001'::uuid, 'post', 'b0000001-0000-0000-0000-000000000005'::uuid, true, NOW()),

-- Notices → spread across all locations
(gen_random_uuid(), 'a0000001-0000-0000-0000-000000000001'::uuid, 'post', 'b0000002-0000-0000-0000-000000000001'::uuid, true, NOW()),
(gen_random_uuid(), 'a0000001-0000-0000-0000-000000000001'::uuid, 'post', 'b0000002-0000-0000-0000-000000000002'::uuid, true, NOW()),
(gen_random_uuid(), 'a0000001-0000-0000-0000-000000000003'::uuid, 'post', 'b0000002-0000-0000-0000-000000000003'::uuid, true, NOW()),
(gen_random_uuid(), 'a0000001-0000-0000-0000-000000000004'::uuid, 'post', 'b0000002-0000-0000-0000-000000000004'::uuid, true, NOW()),
(gen_random_uuid(), 'a0000001-0000-0000-0000-000000000001'::uuid, 'post', 'b0000002-0000-0000-0000-000000000005'::uuid, true, NOW()),
(gen_random_uuid(), 'a0000001-0000-0000-0000-000000000002'::uuid, 'post', 'b0000002-0000-0000-0000-000000000006'::uuid, true, NOW()),
(gen_random_uuid(), 'a0000001-0000-0000-0000-000000000005'::uuid, 'post', 'b0000002-0000-0000-0000-000000000007'::uuid, true, NOW()),
(gen_random_uuid(), 'a0000001-0000-0000-0000-000000000003'::uuid, 'post', 'b0000002-0000-0000-0000-000000000008'::uuid, true, NOW()),
(gen_random_uuid(), 'a0000001-0000-0000-0000-000000000001'::uuid, 'post', 'b0000002-0000-0000-0000-000000000009'::uuid, true, NOW()),
(gen_random_uuid(), 'a0000001-0000-0000-0000-000000000004'::uuid, 'post', 'b0000002-0000-0000-0000-000000000010'::uuid, true, NOW()),

-- Exchanges → Locations 1-3
(gen_random_uuid(), 'a0000001-0000-0000-0000-000000000001'::uuid, 'post', 'b0000003-0000-0000-0000-000000000001'::uuid, true, NOW()),
(gen_random_uuid(), 'a0000001-0000-0000-0000-000000000001'::uuid, 'post', 'b0000003-0000-0000-0000-000000000002'::uuid, true, NOW()),
(gen_random_uuid(), 'a0000001-0000-0000-0000-000000000003'::uuid, 'post', 'b0000003-0000-0000-0000-000000000003'::uuid, true, NOW()),
(gen_random_uuid(), 'a0000001-0000-0000-0000-000000000002'::uuid, 'post', 'b0000003-0000-0000-0000-000000000004'::uuid, true, NOW()),
(gen_random_uuid(), 'a0000001-0000-0000-0000-000000000002'::uuid, 'post', 'b0000003-0000-0000-0000-000000000005'::uuid, true, NOW()),

-- Events → Locations 1-4
(gen_random_uuid(), 'a0000001-0000-0000-0000-000000000001'::uuid, 'post', 'b0000004-0000-0000-0000-000000000001'::uuid, true, NOW()),
(gen_random_uuid(), 'a0000001-0000-0000-0000-000000000003'::uuid, 'post', 'b0000004-0000-0000-0000-000000000002'::uuid, true, NOW()),
(gen_random_uuid(), 'a0000001-0000-0000-0000-000000000001'::uuid, 'post', 'b0000004-0000-0000-0000-000000000003'::uuid, true, NOW()),
(gen_random_uuid(), 'a0000001-0000-0000-0000-000000000004'::uuid, 'post', 'b0000004-0000-0000-0000-000000000004'::uuid, true, NOW()),
(gen_random_uuid(), 'a0000001-0000-0000-0000-000000000005'::uuid, 'post', 'b0000004-0000-0000-0000-000000000005'::uuid, true, NOW()),

-- Spotlights → Locations 1-2
(gen_random_uuid(), 'a0000001-0000-0000-0000-000000000001'::uuid, 'post', 'b0000005-0000-0000-0000-000000000001'::uuid, true, NOW()),
(gen_random_uuid(), 'a0000001-0000-0000-0000-000000000002'::uuid, 'post', 'b0000005-0000-0000-0000-000000000002'::uuid, true, NOW()),
(gen_random_uuid(), 'a0000001-0000-0000-0000-000000000001'::uuid, 'post', 'b0000005-0000-0000-0000-000000000003'::uuid, true, NOW()),

-- References → Location 1
(gen_random_uuid(), 'a0000001-0000-0000-0000-000000000001'::uuid, 'post', 'b0000006-0000-0000-0000-000000000001'::uuid, true, NOW()),
(gen_random_uuid(), 'a0000001-0000-0000-0000-000000000001'::uuid, 'post', 'b0000006-0000-0000-0000-000000000002'::uuid, true, NOW()),
(gen_random_uuid(), 'a0000001-0000-0000-0000-000000000001'::uuid, 'post', 'b0000006-0000-0000-0000-000000000003'::uuid, true, NOW())
ON CONFLICT DO NOTHING;
