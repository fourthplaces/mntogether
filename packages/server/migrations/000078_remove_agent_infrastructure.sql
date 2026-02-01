-- Remove agent infrastructure
-- Agents are replaced by static discovery queries in code

-- Remove foreign key columns from all tables that reference agents
ALTER TABLE websites DROP COLUMN IF EXISTS agent_id;
ALTER TABLE websites DROP COLUMN IF EXISTS tavily_relevance_score;
ALTER TABLE websites DROP COLUMN IF EXISTS tavily_search_metadata;
ALTER TABLE listings DROP COLUMN IF EXISTS agent_id;

-- Drop agent tables (order matters due to foreign keys)
DROP TABLE IF EXISTS agent_crawl_keywords;
DROP TABLE IF EXISTS agents;
