-- Drop curator agent infrastructure tables (agent assistant tables kept for chat)
-- and create standalone search_queries table.

-- Drop in dependency order (child tables first)
DROP TABLE IF EXISTS agent_run_stats;
DROP TABLE IF EXISTS agent_runs;
DROP TABLE IF EXISTS agent_websites;
DROP TABLE IF EXISTS agent_required_tag_kinds;
DROP TABLE IF EXISTS agent_filter_rules;
DROP TABLE IF EXISTS agent_search_queries;
DROP TABLE IF EXISTS agent_curator_configs;

-- Create standalone search_queries table (not tied to agents)
CREATE TABLE search_queries (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    query_text TEXT NOT NULL,
    is_active BOOLEAN NOT NULL DEFAULT true,
    sort_order INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
