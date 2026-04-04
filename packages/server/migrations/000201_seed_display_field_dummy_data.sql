-- Seed dummy data for display-only fields so we can visually review the admin UI.
-- Safe to revert: just NULL out the columns on these rows.
-- NOTE: FK references removed — the referenced posts only existed in the old dev DB dump,
-- not in migration-seeded data. Coordinates are safe to set on any existing post.

-- Food Shelf Hours Extended Through April → coordinates only
UPDATE posts SET latitude = 44.9778, longitude = -93.2650
WHERE id = '83f83a31-73b8-4fc7-9e8f-85d61f1b765b';

-- Anoka County Housing → coordinates only (revision_of FK removed — target doesn't exist)
UPDATE posts SET latitude = 45.1979, longitude = -93.3532
WHERE id = 'b0000001-0000-0000-0000-000000000001';

-- Room Available North Mpls → coordinates only (translation_of FK removed — target doesn't exist)
UPDATE posts SET latitude = 44.9631, longitude = -93.2680
WHERE id = '83918657-1417-40e8-b013-d4cb28723509';
