-- Remove AI-generated columns (embeddings, relevance scoring).
-- All AI features now handled externally by Root Signal.

ALTER TABLE posts DROP COLUMN IF EXISTS embedding;
ALTER TABLE posts DROP COLUMN IF EXISTS relevance_score;
ALTER TABLE posts DROP COLUMN IF EXISTS relevance_breakdown;
ALTER TABLE members DROP COLUMN IF EXISTS embedding;
ALTER TABLE notes DROP COLUMN IF EXISTS embedding;
