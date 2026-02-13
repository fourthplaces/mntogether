-- Add relevance scoring columns to posts for human review triage.
-- Composite score (1-10) computed from immigration relevance (50%),
-- actionability (30%), and completeness (20%).

ALTER TABLE posts ADD COLUMN relevance_score INTEGER;
ALTER TABLE posts ADD COLUMN relevance_breakdown TEXT;
ALTER TABLE posts ADD COLUMN scored_at TIMESTAMP WITH TIME ZONE;

CREATE INDEX idx_posts_relevance_score ON posts(relevance_score)
    WHERE relevance_score IS NOT NULL;

COMMENT ON COLUMN posts.relevance_score IS 'Composite relevance score 1-10 (immigration relevance 50%, actionability 30%, completeness 20%)';
COMMENT ON COLUMN posts.relevance_breakdown IS 'Human-readable per-factor breakdown of the relevance score';
COMMENT ON COLUMN posts.scored_at IS 'When the relevance score was last computed';
