-- Add revision_of_post_id to posts table for draft revision support
-- When AI generates updates, they create revision posts for user review before committing

ALTER TABLE posts ADD COLUMN revision_of_post_id UUID REFERENCES posts(id);
CREATE INDEX idx_posts_revision_of_post_id ON posts(revision_of_post_id);
