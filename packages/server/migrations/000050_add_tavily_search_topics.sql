-- Migration 000050: Add Tavily Search Integration Tables
--
-- This migration adds support for automated discovery of community resources
-- using Tavily search API. Search topics are configurable and run on schedules.

-- Create search_topics table
CREATE TABLE search_topics (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name TEXT NOT NULL,
    query_template TEXT NOT NULL,  -- e.g., "legal aid immigrants {location}"
    description TEXT,
    enabled BOOL NOT NULL DEFAULT true,
    search_frequency_hours INT NOT NULL DEFAULT 24,
    last_searched_at TIMESTAMPTZ,
    location_context TEXT DEFAULT 'Twin Cities, Minnesota',
    service_area_tags TEXT[] DEFAULT '{}',
    search_depth TEXT DEFAULT 'basic',  -- 'basic' or 'advanced'
    max_results INT DEFAULT 5,
    days_range INT DEFAULT 7,  -- Search results from last N days
    min_relevance_score NUMERIC(3,2) DEFAULT 0.5,  -- Filter threshold (0.0-1.0)
    total_searches_run INT DEFAULT 0,
    total_domains_discovered INT DEFAULT 0,
    total_domains_approved INT DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_by UUID REFERENCES members(id),
    CONSTRAINT unique_topic_name UNIQUE(name),
    CONSTRAINT valid_relevance_score CHECK (min_relevance_score BETWEEN 0 AND 1)
);

-- Indexes for efficient queries
CREATE INDEX idx_search_topics_enabled ON search_topics(enabled);
CREATE INDEX idx_search_topics_due ON search_topics(last_searched_at) WHERE enabled = true;

-- Extend domains table to track search discovery
ALTER TABLE domains
  ADD COLUMN discovered_via_search_topic_id UUID REFERENCES search_topics(id),
  ADD COLUMN tavily_relevance_score NUMERIC(3,2),
  ADD COLUMN tavily_search_metadata JSONB DEFAULT '{}';

CREATE INDEX idx_domains_search_topic ON domains(discovered_via_search_topic_id);

-- Add comment documentation
COMMENT ON TABLE search_topics IS 'Configurable search topics for automated resource discovery via Tavily API';
COMMENT ON COLUMN domains.discovered_via_search_topic_id IS 'Links domain to the search topic that discovered it';
COMMENT ON COLUMN domains.tavily_relevance_score IS 'Relevance score from Tavily API (0.0-1.0)';
COMMENT ON COLUMN domains.tavily_search_metadata IS 'Additional metadata from Tavily search result (published_date, etc.)';
