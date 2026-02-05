-- Discovery domain tables
-- Manages search queries, filter rules, and discovery run tracking

-- Search queries for Tavily-based website discovery
CREATE TABLE discovery_queries (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    query_text TEXT NOT NULL,
    category TEXT,
    is_active BOOLEAN NOT NULL DEFAULT true,
    created_by UUID REFERENCES members(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Plain-text filter rules evaluated by AI before websites enter approval queue
-- query_id = NULL means global rule (applies to all queries)
CREATE TABLE discovery_filter_rules (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    query_id UUID REFERENCES discovery_queries(id) ON DELETE CASCADE,
    rule_text TEXT NOT NULL,
    sort_order INT NOT NULL DEFAULT 0,
    is_active BOOLEAN NOT NULL DEFAULT true,
    created_by UUID REFERENCES members(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_discovery_filter_rules_query_id ON discovery_filter_rules(query_id);
CREATE INDEX idx_discovery_filter_rules_global ON discovery_filter_rules(query_id) WHERE query_id IS NULL;

-- Tracks each discovery run (scheduled or manual)
CREATE TABLE discovery_runs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    queries_executed INT NOT NULL DEFAULT 0,
    total_results INT NOT NULL DEFAULT 0,
    websites_created INT NOT NULL DEFAULT 0,
    websites_filtered INT NOT NULL DEFAULT 0,
    started_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at TIMESTAMPTZ,
    trigger_type TEXT NOT NULL DEFAULT 'manual'
);

-- Individual results from each discovery run (full lineage)
CREATE TABLE discovery_run_results (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    run_id UUID NOT NULL REFERENCES discovery_runs(id) ON DELETE CASCADE,
    query_id UUID NOT NULL REFERENCES discovery_queries(id),
    domain TEXT NOT NULL,
    url TEXT NOT NULL,
    title TEXT,
    snippet TEXT,
    relevance_score DOUBLE PRECISION,
    filter_result TEXT NOT NULL DEFAULT 'pending',
    filter_reason TEXT,
    website_id UUID REFERENCES websites(id),
    discovered_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_discovery_run_results_run_id ON discovery_run_results(run_id);
CREATE INDEX idx_discovery_run_results_query_id ON discovery_run_results(query_id);
CREATE INDEX idx_discovery_run_results_website_id ON discovery_run_results(website_id);
CREATE INDEX idx_discovery_run_results_domain ON discovery_run_results(domain);
