-- Migrate existing post_locations data into the polymorphic locationables table.
-- Does NOT drop post_locations — kept for rollback safety.
INSERT INTO locationables (location_id, locatable_type, locatable_id, is_primary, notes, added_at)
SELECT location_id, 'post', post_id, is_primary, notes, created_at
FROM post_locations
ON CONFLICT (location_id, locatable_type, locatable_id) DO NOTHING;
