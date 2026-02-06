-- Invert container FK: posts.comments_container_id references containers(id).
-- Only posts use containers now (organizations removed), so we move the FK
-- to the posts side and drop the polymorphic columns from containers.

-- Add FK column on posts
ALTER TABLE posts ADD COLUMN comments_container_id UUID REFERENCES containers(id);

-- Backfill from existing container_type + entity_id
UPDATE posts p SET comments_container_id = c.id
FROM containers c WHERE c.container_type = 'post_comments' AND c.entity_id = p.id;

-- Drop polymorphic columns from containers
ALTER TABLE containers DROP COLUMN container_type;
ALTER TABLE containers DROP COLUMN entity_id;

-- Drop the old index
DROP INDEX IF EXISTS idx_containers_entity;

-- Add index on the new FK column
CREATE INDEX idx_posts_comments_container ON posts(comments_container_id) WHERE comments_container_id IS NOT NULL;
