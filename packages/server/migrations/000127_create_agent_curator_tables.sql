-- Curator-specific tables for the agent pipeline.

CREATE TABLE agent_search_queries (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    agent_id UUID NOT NULL REFERENCES agents(id) ON DELETE CASCADE,
    query_text TEXT NOT NULL,
    is_active BOOLEAN NOT NULL DEFAULT true,
    sort_order INT NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX idx_agent_search_queries_agent_id ON agent_search_queries(agent_id);

CREATE TABLE agent_filter_rules (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    agent_id UUID NOT NULL REFERENCES agents(id) ON DELETE CASCADE,
    rule_text TEXT NOT NULL,
    is_active BOOLEAN NOT NULL DEFAULT true,
    sort_order INT NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX idx_agent_filter_rules_agent_id ON agent_filter_rules(agent_id);

CREATE TABLE agent_websites (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    agent_id UUID NOT NULL REFERENCES agents(id) ON DELETE CASCADE,
    website_id UUID NOT NULL REFERENCES websites(id) ON DELETE CASCADE,
    discovered_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(agent_id, website_id)
);
CREATE INDEX idx_agent_websites_agent_id ON agent_websites(agent_id);
CREATE INDEX idx_agent_websites_website_id ON agent_websites(website_id);

CREATE TABLE agent_runs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    agent_id UUID NOT NULL REFERENCES agents(id) ON DELETE CASCADE,
    step TEXT NOT NULL,
    trigger_type TEXT NOT NULL DEFAULT 'manual',
    status TEXT NOT NULL DEFAULT 'running',
    started_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at TIMESTAMPTZ
);
CREATE INDEX idx_agent_runs_agent_id ON agent_runs(agent_id);

CREATE TABLE agent_run_stats (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    run_id UUID NOT NULL REFERENCES agent_runs(id) ON DELETE CASCADE,
    stat_key TEXT NOT NULL,
    stat_value INT NOT NULL DEFAULT 0
);
CREATE INDEX idx_agent_run_stats_run_id ON agent_run_stats(run_id);

CREATE TABLE agent_required_tag_kinds (
    agent_id UUID NOT NULL REFERENCES agents(id) ON DELETE CASCADE,
    tag_kind_id UUID NOT NULL REFERENCES tag_kinds(id) ON DELETE CASCADE,
    PRIMARY KEY (agent_id, tag_kind_id)
);
