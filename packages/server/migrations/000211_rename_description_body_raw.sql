-- =============================================================================
-- Consolidate post body fields: rename description → body_raw, drop dead columns
-- =============================================================================
-- description      → body_raw   (the raw source text from Root Signal or user input)
-- description_markdown           (dead — 1/154 posts, replaced by body_ast)
-- summary                        (dead — replaced by body_light from Root Signal)
--
-- Remaining body model:
--   body_raw    text NOT NULL   — source text, always present
--   body_ast    jsonb           — Plate.js rich editor content
--   body_heavy  text            — feature templates (from Root Signal)
--   body_medium text            — gazette/bulletin templates (from Root Signal)
--   body_light  text            — ticker/digest/cards (from Root Signal)
-- =============================================================================

ALTER TABLE posts RENAME COLUMN description TO body_raw;
ALTER TABLE posts DROP COLUMN IF EXISTS description_markdown;
ALTER TABLE posts DROP COLUMN IF EXISTS summary;

-- Update search_vector trigger to reference body_raw instead of description
CREATE OR REPLACE FUNCTION posts_search_vector_update() RETURNS trigger AS $$
BEGIN
    NEW.search_vector :=
        setweight(to_tsvector('english', COALESCE(NEW.title, '')), 'A') ||
        setweight(to_tsvector('english', COALESCE(NEW.body_raw, '')), 'B');
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Backfill existing rows
UPDATE posts SET search_vector =
    setweight(to_tsvector('english', COALESCE(title, '')), 'A') ||
    setweight(to_tsvector('english', COALESCE(body_raw, '')), 'B');
