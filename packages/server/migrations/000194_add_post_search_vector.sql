-- Full-text search + trigram fuzzy matching for posts.
-- Replaces ILIKE '%query%' with ranked tsvector search and typo-tolerant trigram matching.

-- Enable trigram extension for fuzzy/typo-tolerant search
CREATE EXTENSION IF NOT EXISTS pg_trgm;

-- Add stored tsvector column for full-text search
ALTER TABLE posts ADD COLUMN search_vector tsvector;

-- Populate from existing data (title weighted A, description weighted B)
UPDATE posts SET search_vector =
    setweight(to_tsvector('english', COALESCE(title, '')), 'A') ||
    setweight(to_tsvector('english', COALESCE(description, '')), 'B');

-- GIN index for full-text search
CREATE INDEX idx_posts_search_vector ON posts USING GIN(search_vector);

-- Trigram GIN index for fuzzy matching on title
CREATE INDEX idx_posts_title_trgm ON posts USING GIN(title gin_trgm_ops);

-- Auto-update trigger: keeps search_vector in sync on INSERT or UPDATE of title/description
CREATE OR REPLACE FUNCTION posts_search_vector_update() RETURNS trigger AS $$
BEGIN
    NEW.search_vector :=
        setweight(to_tsvector('english', COALESCE(NEW.title, '')), 'A') ||
        setweight(to_tsvector('english', COALESCE(NEW.description, '')), 'B');
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trg_posts_search_vector
    BEFORE INSERT OR UPDATE OF title, description ON posts
    FOR EACH ROW EXECUTE FUNCTION posts_search_vector_update();
