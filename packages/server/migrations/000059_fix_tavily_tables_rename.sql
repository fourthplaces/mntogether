-- Fix: Rename domain_research_id to website_research_id in tavily tables
-- This was missed in the 000057 migration

-- tavily_search_queries
ALTER TABLE tavily_search_queries RENAME COLUMN domain_research_id TO website_research_id;

ALTER TABLE tavily_search_queries DROP CONSTRAINT IF EXISTS tavily_search_queries_domain_research_id_fkey;
ALTER TABLE tavily_search_queries
    ADD CONSTRAINT tavily_search_queries_website_research_id_fkey
    FOREIGN KEY (website_research_id) REFERENCES website_research(id) ON DELETE CASCADE;

DROP INDEX IF EXISTS idx_tavily_search_queries_research_id;
CREATE INDEX idx_tavily_search_queries_website_research_id ON tavily_search_queries(website_research_id);
