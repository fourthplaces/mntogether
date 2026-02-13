-- Migration 000054: Fix agent min_relevance_score type
--
-- Change from NUMERIC(3,2) to DOUBLE PRECISION to match Rust f64 type

ALTER TABLE agents
  ALTER COLUMN min_relevance_score TYPE DOUBLE PRECISION;

-- Also fix tavily_relevance_score in domains table
ALTER TABLE domains
  ALTER COLUMN tavily_relevance_score TYPE DOUBLE PRECISION;

-- Add comment
COMMENT ON COLUMN agents.min_relevance_score IS 'Minimum relevance score threshold (0.0-1.0) for filtering search results';
COMMENT ON COLUMN domains.tavily_relevance_score IS 'Relevance score from Tavily API (0.0-1.0)';
