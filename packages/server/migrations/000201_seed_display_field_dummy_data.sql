-- Seed dummy data for display-only fields so we can visually review the admin UI.
-- Safe to revert: just NULL out the columns on these rows.

-- Food Shelf Hours Extended Through April → coordinates only
UPDATE posts SET latitude = 44.9778, longitude = -93.2650
WHERE id = '83f83a31-73b8-4fc7-9e8f-85d61f1b765b';

-- Anoka County Housing → coordinates + revision_of link
UPDATE posts SET latitude = 45.1979, longitude = -93.3532,
  revision_of_post_id = '83f83a31-73b8-4fc7-9e8f-85d61f1b765b'
WHERE id = 'b0000001-0000-0000-0000-000000000001';

-- Room Available North Mpls → coordinates + translation_of link
UPDATE posts SET latitude = 44.9631, longitude = -93.2680,
  translation_of_id = '83f83a31-73b8-4fc7-9e8f-85d61f1b765b'
WHERE id = '83918657-1417-40e8-b013-d4cb28723509';

-- Emergency Assistance Dakota → duplicate_of link
UPDATE posts SET duplicate_of_id = 'e7e41fb0-b707-4800-937f-ec4c13cdaea1'
WHERE id = '831c0060-af63-4f2b-bb3f-c0aa8356e2ad';
