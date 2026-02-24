-- Migration 000052: Migrate search_topics to agents
--
-- Unifies search discovery (Tavily) with extraction instructions (AI)
-- Agents now handle the complete autonomous pipeline:
-- 1. Search via Tavily
-- 2. Auto-scrape discovered domains via Firecrawl
-- 3. Extract listings via AI with agent-specific instructions
-- 4. Auto-approve domains when listings are found

-- Rename search_topics table to agents
ALTER TABLE search_topics RENAME TO agents;

-- Rename indexes and constraints
ALTER INDEX idx_search_topics_enabled RENAME TO idx_agents_enabled;
ALTER INDEX idx_search_topics_due RENAME TO idx_agents_due;
ALTER TABLE agents RENAME CONSTRAINT unique_topic_name TO unique_agent_name;

-- Add extraction and automation fields
ALTER TABLE agents
  ADD COLUMN extraction_instructions TEXT,
  ADD COLUMN system_prompt TEXT,
  ADD COLUMN auto_approve_domains BOOL NOT NULL DEFAULT true,
  ADD COLUMN auto_scrape BOOL NOT NULL DEFAULT true,
  ADD COLUMN auto_create_listings BOOL NOT NULL DEFAULT true;

-- Update domains table references
ALTER TABLE domains
  RENAME COLUMN discovered_via_search_topic_id TO agent_id;

ALTER INDEX idx_domains_search_topic RENAME TO idx_domains_agent;

-- Add agent_id to listings table to track which agent extracted each listing
ALTER TABLE listings
  ADD COLUMN agent_id UUID REFERENCES agents(id) ON DELETE SET NULL;

CREATE INDEX idx_listings_agent ON listings(agent_id);

-- Update column comments
COMMENT ON TABLE agents IS 'Autonomous agents that search (Tavily), scrape (Firecrawl), and extract listings (AI)';
COMMENT ON COLUMN agents.name IS 'Agent name (e.g., "Legal Aid Finder", "Volunteer Opportunities")';
COMMENT ON COLUMN agents.query_template IS 'Tavily search query template with {location} placeholder';
COMMENT ON COLUMN agents.extraction_instructions IS 'Instructions for AI extraction (what to look for)';
COMMENT ON COLUMN agents.system_prompt IS 'Detailed system prompt for AI extraction';
COMMENT ON COLUMN agents.auto_approve_domains IS 'Auto-approve domains when listings are extracted';
COMMENT ON COLUMN agents.auto_scrape IS 'Automatically scrape discovered domains';
COMMENT ON COLUMN agents.auto_create_listings IS 'Automatically create listings from extractions';
COMMENT ON COLUMN domains.agent_id IS 'Agent that discovered this domain via Tavily search';
COMMENT ON COLUMN listings.agent_id IS 'Agent that extracted this listing';

-- Set default extraction instructions for existing agents (if any)
UPDATE agents
SET extraction_instructions = 'Extract community resources, services, and volunteer opportunities. Include eligibility requirements, contact information, and how to access the service.'
WHERE extraction_instructions IS NULL;

UPDATE agents
SET system_prompt = 'You are an expert at identifying community resources and services. Extract detailed information about programs, eligibility, contact details, and how people can access help.'
WHERE system_prompt IS NULL;
