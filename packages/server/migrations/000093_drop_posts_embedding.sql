-- Drop embedding column from posts table
--
-- Embeddings are no longer used for post deduplication.
-- LLM-based deduplication now handles semantic duplicate detection.

ALTER TABLE posts DROP COLUMN IF EXISTS embedding;

-- Also drop any indexes on the embedding column
DROP INDEX IF EXISTS posts_embedding_idx;
DROP INDEX IF EXISTS idx_posts_embedding;
