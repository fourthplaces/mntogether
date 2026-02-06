-- Drop the polymorphic contacts table.
-- post_contacts (migration 000092) is the only contact table needed.
-- Organizations and providers are removed or use their own patterns.

-- Migrate any remaining post contacts from polymorphic table
INSERT INTO post_contacts (post_id, contact_type, contact_value, contact_label, display_order)
SELECT contactable_id, contact_type, contact_value, contact_label, display_order
FROM contacts WHERE contactable_type = 'post'
ON CONFLICT DO NOTHING;

-- Drop the polymorphic contacts table
DROP TABLE IF EXISTS contacts;
