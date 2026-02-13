ALTER TABLE posts ADD COLUMN duplicate_of_id UUID REFERENCES posts(id) ON DELETE SET NULL;
CREATE INDEX idx_posts_duplicate_of_id ON posts(duplicate_of_id) WHERE duplicate_of_id IS NOT NULL;
