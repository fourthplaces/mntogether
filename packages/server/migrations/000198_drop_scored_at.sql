-- scored_at was part of the AI relevance scoring system dropped in 000193.
-- This column was missed in that migration.
ALTER TABLE posts DROP COLUMN IF EXISTS scored_at;
