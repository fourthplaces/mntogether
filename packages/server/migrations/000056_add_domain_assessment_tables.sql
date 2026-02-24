-- Table 1: Research Session Metadata
CREATE TABLE domain_research (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    domain_id UUID NOT NULL REFERENCES domains(id) ON DELETE CASCADE,

    homepage_url TEXT NOT NULL,
    homepage_fetched_at TIMESTAMP WITH TIME ZONE NOT NULL,
    tavily_searches_completed_at TIMESTAMP WITH TIME ZONE,

    created_by UUID REFERENCES members(id),
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

CREATE INDEX idx_domain_research_domain_id ON domain_research(domain_id);
CREATE INDEX idx_domain_research_created_at ON domain_research(created_at DESC);

-- Table 2: Homepage Content
CREATE TABLE domain_research_homepage (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    domain_research_id UUID NOT NULL REFERENCES domain_research(id) ON DELETE CASCADE,

    html TEXT,
    markdown TEXT,

    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

CREATE INDEX idx_domain_research_homepage_research_id ON domain_research_homepage(domain_research_id);

-- Table 3: Tavily Search Queries (one per search)
CREATE TABLE tavily_search_queries (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    domain_research_id UUID NOT NULL REFERENCES domain_research(id) ON DELETE CASCADE,

    query TEXT NOT NULL,
    search_depth VARCHAR(20), -- 'basic' or 'advanced'
    max_results INTEGER,
    days_filter INTEGER,

    executed_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

CREATE INDEX idx_tavily_search_queries_research_id ON tavily_search_queries(domain_research_id);

-- Table 4: Tavily Search Results (one row per result)
CREATE TABLE tavily_search_results (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    query_id UUID NOT NULL REFERENCES tavily_search_queries(id) ON DELETE CASCADE,

    title TEXT NOT NULL,
    url TEXT NOT NULL,
    content TEXT NOT NULL,
    score DECIMAL(3,2) NOT NULL, -- Relevance score 0.00-1.00
    published_date TEXT, -- ISO date string

    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

CREATE INDEX idx_tavily_search_results_query_id ON tavily_search_results(query_id);
CREATE INDEX idx_tavily_search_results_score ON tavily_search_results(score DESC);

-- Table 5: Domain Assessments (AI-generated reports)
CREATE TABLE domain_assessments (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    domain_id UUID NOT NULL REFERENCES domains(id) ON DELETE CASCADE,
    domain_research_id UUID REFERENCES domain_research(id) ON DELETE SET NULL,

    -- AI-generated content (markdown)
    assessment_markdown TEXT NOT NULL,

    -- Structured metadata
    recommendation VARCHAR(50) NOT NULL, -- 'approve', 'reject', 'needs_review'
    confidence_score DECIMAL(3,2), -- 0.00 to 1.00
    organization_name TEXT,
    founded_year INTEGER,

    -- Generation metadata
    generated_by UUID REFERENCES members(id),
    generated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    model_used VARCHAR(100) NOT NULL, -- 'gpt-4-turbo'

    -- Review tracking
    reviewed_by_human BOOLEAN DEFAULT FALSE,
    human_notes TEXT,

    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

CREATE INDEX idx_domain_assessments_domain_id ON domain_assessments(domain_id);
CREATE INDEX idx_domain_assessments_research_id ON domain_assessments(domain_research_id);
CREATE INDEX idx_domain_assessments_recommendation ON domain_assessments(recommendation);
CREATE INDEX idx_domain_assessments_generated_at ON domain_assessments(generated_at DESC);
