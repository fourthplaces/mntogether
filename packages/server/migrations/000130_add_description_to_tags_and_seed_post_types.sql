-- Add description column to tags
ALTER TABLE tags ADD COLUMN description TEXT;

COMMENT ON COLUMN tags.description IS 'Optional description of the tag purpose';

-- Seed post_type tags for home page buckets
INSERT INTO tags (kind, value, display_name, description) VALUES
    ('post_type', 'seeking', 'I Need Help', 'Find resources, services, and support available to you'),
    ('post_type', 'offering', 'I Want to Support', 'Discover ways to volunteer, donate, or contribute'),
    ('post_type', 'announcement', 'Community Bulletin', 'Stay informed about community news and events')
ON CONFLICT (kind, value) DO UPDATE SET
    display_name = EXCLUDED.display_name,
    description = EXCLUDED.description;
