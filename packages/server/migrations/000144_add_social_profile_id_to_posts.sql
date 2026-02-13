ALTER TABLE posts
    ADD COLUMN social_profile_id UUID REFERENCES social_profiles(id);

CREATE INDEX idx_posts_social_profile_id ON posts(social_profile_id);
