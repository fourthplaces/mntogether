ALTER TABLE posts ADD COLUMN translation_of_id UUID REFERENCES posts(id);
CREATE INDEX idx_posts_translation_of_id ON posts(translation_of_id);
