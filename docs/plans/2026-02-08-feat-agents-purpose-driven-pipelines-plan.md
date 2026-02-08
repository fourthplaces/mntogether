---
title: "feat: Agents — Purpose-Driven Pipelines"
type: feat
date: 2026-02-08
---

# Agents — Purpose-Driven Pipelines

## Overview

An **agent** is an autonomous entity with a member identity. Agents have **roles** that define what they do:

- **`assistant`** — responds to users in chat (existing chatbot functionality)
- **`curator`** — discovers websites, extracts posts, enriches them, and keeps them up to date

All agents share a base identity (display_name, member, status) with role-specific configuration in separate tables. This unifies the existing chatbot agents and the new content curation pipelines under one concept.

## Problem Statement

Posts are being extracted that are unrelated to what we care about. The root cause: extraction prompts are hardcoded and one-size-fits-all. A page about a bakery either gets shoehorned into "community resources" framing or gets ignored. Meanwhile, discovery queries are already database-driven but disconnected from extraction — there's no concept of *purpose* flowing through the pipeline.

Additionally, the existing chatbot "agents" and the new pipeline concept are both autonomous entities that act on behalf of the system — they should be modeled as the same thing with different roles.

## Proposed Solution

Restructure the `agents` table into a base table with role-specific config tables. Curator agents own their full lifecycle:

1. **Discover** — search the web for relevant websites using agent-specific queries
2. **Extract** — create posts using agent-specific extraction instructions
3. **Enrich** — investigate posts for missing contact info, location, required tags
4. **Monitor** — re-crawl and sync posts (update stale, remove dead)

Each curator's **purpose** (free-text) shapes AI prompts, combined with structured fields (audience roles, required tag kinds) for the mechanical parts.

## Technical Approach

### Architecture

```
Admin UI (Next.js)
  ↓ callService("Agents", "run_agent_step", { agent_id, step })
Restate Service: AgentsService
  ↓ activities (curator role only)
Pipeline Activities:
  discover() → extract() → enrich() → monitor()
  ↓ uses
Existing infrastructure:
  Tavily (search), OpenAI (extraction), Website/Post models
```

### ERD

```mermaid
erDiagram
    agents {
        uuid id PK
        text display_name
        uuid member_id FK
        text role "assistant | curator"
        text status "draft | active | paused"
        timestamptz created_at
        timestamptz updated_at
    }
    agent_assistant_configs {
        uuid agent_id PK_FK
        text preamble
        text config_name
    }
    agent_curator_configs {
        uuid agent_id PK_FK
        text purpose
        text[] audience_roles
        text schedule_discover
        text schedule_monitor
    }
    agent_search_queries {
        uuid id PK
        uuid agent_id FK
        text query_text
        boolean is_active
        int sort_order
        timestamptz created_at
    }
    agent_filter_rules {
        uuid id PK
        uuid agent_id FK
        text rule_text
        boolean is_active
        int sort_order
        timestamptz created_at
    }
    agent_websites {
        uuid id PK
        uuid agent_id FK
        uuid website_id FK
        timestamptz discovered_at
    }
    agent_runs {
        uuid id PK
        uuid agent_id FK
        text step
        text trigger_type
        timestamptz started_at
        timestamptz completed_at
        text status
    }
    agent_run_stats {
        uuid id PK
        uuid run_id FK
        text stat_key
        int stat_value
    }
    agent_required_tag_kinds {
        uuid agent_id FK
        uuid tag_kind_id FK
    }
    posts {
        uuid agent_id FK "nullable - new column"
    }

    agents ||--o| agent_assistant_configs : "role = assistant"
    agents ||--o| agent_curator_configs : "role = curator"
    agents ||--o{ agent_search_queries : has
    agents ||--o{ agent_filter_rules : has
    agents ||--o{ agent_websites : discovers
    agents ||--o{ agent_runs : tracks
    agents ||--o{ agent_required_tag_kinds : requires
    agent_required_tag_kinds }o--|| tag_kinds : references
    agent_runs ||--o{ agent_run_stats : has
    agent_websites }o--|| websites : references
    agents ||--o{ posts : creates
    agents }o--|| members : identity
```

### Implementation Phases

#### Phase 1: Foundation — Schema + Models

**Migration: `000126_restructure_agents.sql`**

Restructure the existing agents table into base + role-specific configs:

```sql
-- Step 1: Add role column to existing agents table
ALTER TABLE agents ADD COLUMN role TEXT NOT NULL DEFAULT 'assistant';
ALTER TABLE agents ADD CONSTRAINT agents_role_check CHECK (role IN ('assistant', 'curator'));

-- Step 2: Move assistant-specific fields to config table
CREATE TABLE agent_assistant_configs (
    agent_id UUID PRIMARY KEY REFERENCES agents(id) ON DELETE CASCADE,
    preamble TEXT NOT NULL DEFAULT '',
    config_name TEXT NOT NULL DEFAULT 'admin'
);

-- Step 3: Migrate existing data into assistant configs
INSERT INTO agent_assistant_configs (agent_id, preamble, config_name)
SELECT id, preamble, config_name FROM agents;

-- Step 4: Drop assistant-specific columns from base table
ALTER TABLE agents DROP COLUMN preamble;
ALTER TABLE agents DROP COLUMN config_name;

-- Step 5: Add status column (replaces is_active for all roles)
ALTER TABLE agents ADD COLUMN status TEXT NOT NULL DEFAULT 'active';
ALTER TABLE agents ADD CONSTRAINT agents_status_check CHECK (status IN ('draft', 'active', 'paused'));
UPDATE agents SET status = CASE WHEN is_active THEN 'active' ELSE 'paused' END;
ALTER TABLE agents DROP COLUMN is_active;

-- Step 6: Recreate unique index on assistant config table
-- Original was partial (WHERE is_active = true), but status now lives on agents table
-- and partial indexes can't cross tables. Global uniqueness is fine here —
-- there are only 2 assistant configs ("admin", "public") and duplicates
-- would be a bug regardless of status.
DROP INDEX IF EXISTS idx_agents_config_name;
CREATE UNIQUE INDEX idx_agent_assistant_configs_config_name
    ON agent_assistant_configs(config_name);

-- Step 7: Create curator config table
CREATE TABLE agent_curator_configs (
    agent_id UUID PRIMARY KEY REFERENCES agents(id) ON DELETE CASCADE,
    purpose TEXT NOT NULL DEFAULT '',
    audience_roles TEXT[] NOT NULL DEFAULT '{}',
    schedule_discover TEXT,
    schedule_monitor TEXT
);
```

**Migration: `000127_create_agent_curator_tables.sql`**

Create curator-specific tables:

```sql
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
```

**Migration: `000128_add_agent_id_to_posts.sql`**

```sql
ALTER TABLE posts ADD COLUMN agent_id UUID REFERENCES agents(id);
CREATE INDEX idx_posts_agent_id ON posts(agent_id);
```

Nullable — existing posts have no agent.

**Rust domain structure:**

```
domains/agents/              # restructured
  mod.rs
  models/
    agent.rs                 # Agent base struct + CRUD (shared across roles)
    assistant_config.rs      # AgentAssistantConfig struct + queries
    curator_config.rs        # AgentCuratorConfig struct + queries
    agent_search_query.rs    # AgentSearchQuery struct + CRUD
    agent_filter_rule.rs     # AgentFilterRule struct + CRUD
    agent_website.rs         # AgentWebsite struct + queries
    agent_run.rs             # AgentRun + AgentRunStat structs + queries
    agent_required_tag_kind.rs # AgentRequiredTagKind struct + queries
    mod.rs
  activities/
    discover.rs              # Tavily search + filter + website creation
    extract.rs               # Purpose-injected extraction
    enrich.rs                # Contact/location/tag investigation
    monitor.rs               # Re-crawl + sync
    evaluate_filter.rs       # AI pre-filter (relocated from discovery)
    mod.rs
  restate/
    services/
      agents.rs              # CRUD + run triggers + scheduling
    mod.rs
```

**Models** (all use `sqlx::query_as::<_, Self>` per CLAUDE.md):

- `Agent`: find_all, find_by_id, find_by_role, find_active, create (provisions synthetic member row with `expo_push_token = "agent:{slug}"`), update, set_status
- `AgentAssistantConfig`: find_by_agent, find_by_config_name, create, update
- `AgentCuratorConfig`: find_by_agent, create, update
- `AgentSearchQuery`: find_by_agent, find_active_by_agent, create, update, delete, toggle_active
- `AgentFilterRule`: find_by_agent, find_active_by_agent, create, update, delete
- `AgentWebsite`: find_by_agent, link (upsert), find_by_website
- `AgentRun`: create, complete, fail, find_by_agent, find_recent
- `AgentRunStat`: create_batch, find_by_run
- `AgentRequiredTagKind`: find_by_agent, set_for_agent (replace all), remove

**Files changed:**
- `packages/server/src/domains/agents/models/agent.rs` — restructure: remove preamble/config_name, add role/status
- `packages/server/src/domains/agents/models/` — new model files for configs and curator tables
- `packages/server/src/domains/agents/mod.rs` — declare new submodules
- All files referencing `Agent.preamble` or `Agent.config_name` — update to use `AgentAssistantConfig`
- `packages/server/src/domains/agents/` — new activities/ and restate/ directories

#### Phase 2: Pipeline Activities

Each activity is a pure async function taking `&ServerDeps`.

**`discover.rs`** — Runs the agent's search queries via Tavily:
1. Load agent + curator config + active search queries
2. For each query, substitute `{location}` and execute Tavily search
3. Deduplicate by domain (skip websites already linked to this agent)
4. AI pre-filter against agent's filter rules (using `evaluate_websites_against_filters` — relocated from discovery domain to `agents/activities/evaluate_filter.rs`, decoupled from `DiscoveryFilterRule` to use `AgentFilterRule`)
5. Create websites that don't exist yet (`Website::find_or_create`)
6. Create `agent_websites` join rows for all passing results
7. Return stats: queries_executed, total_results, websites_created, websites_filtered

**`extract.rs`** — Runs purpose-injected extraction:
1. Load agent + curator config + linked websites (via `agent_websites`)
2. For each website, load crawled pages
3. Load agent's required tag kinds (via `agent_required_tag_kinds` → `tag_kinds`)
4. Build extraction prompt by injecting curator's `purpose` and required tags into the template:
   ```
   You are extracting {curator_config.purpose} from website content.

   Focus on opportunities relevant to these audiences: {curator_config.audience_roles.join(", ")}

   REQUIRED INFORMATION: Every post MUST be classified with these tag kinds:
   - {tag_kind.slug}: {tag_kind.description} — from: {existing values}
   (for each required tag kind)

   {existing format/rules from NARRATIVE_EXTRACTION_PROMPT}
   ```
5. Run 3-pass extraction (narrative → dedupe → investigate) using modified prompts
6. Create posts with `agent_id` set
7. Return stats: websites_processed, posts_extracted

**`enrich.rs`** — Investigates posts for missing data:
1. Load agent's required tag kinds
2. Load agent's posts that are missing contact info, location, OR required tags
3. Run agentic investigation with required tag kinds injected: "This post is missing: {missing_tag_kinds}. Investigate and find this information."
4. Update posts with findings, apply discovered tags
5. Return stats: posts_enriched, contacts_found, locations_found, posts_still_missing_tags

**Note:** Post completeness is **derived at query time** by checking whether all agent-required tag kinds are present — not a stored status. The existing post status constraint (`pending_approval|active|filled|rejected|expired`) is unchanged. The admin UI shows a "missing tags" indicator on posts that lack required tag kinds.

**`monitor.rs`** — Re-crawl and sync:
1. Load agent's websites (via `agent_websites`)
2. Trigger re-crawl for each website
3. Run LLM sync scoped to this agent's posts only (critical: `WHERE agent_id = $1`)
4. Create sync proposals for human review
5. Return stats: websites_crawled, posts_updated, posts_inserted, posts_stale

**Files changed/created:**
- `packages/server/src/domains/agents/activities/discover.rs` — new
- `packages/server/src/domains/agents/activities/extract.rs` — new
- `packages/server/src/domains/agents/activities/enrich.rs` — new
- `packages/server/src/domains/agents/activities/monitor.rs` — new
- `packages/server/src/domains/agents/activities/evaluate_filter.rs` — new (relocated from `discovery/activities/evaluate_filter.rs`, decoupled from DiscoveryFilterRule)
- `packages/server/src/domains/agents/activities/mod.rs` — new
- `packages/server/src/domains/crawling/activities/post_extraction.rs` — refactor to accept purpose parameter instead of hardcoded prompt
- `packages/server/src/domains/posts/activities/create_post.rs` — accept optional `agent_id`

#### Phase 3: Restate Service

**`AgentsService`** — stateless Restate service handling CRUD + pipeline triggers:

```rust
#[restate_sdk::service]
#[name = "Agents"]
pub trait AgentsService {
    // CRUD - Agent (all roles)
    // create_agent provisions a synthetic member row (same pattern as existing assistant creation):
    //   1. INSERT INTO members (expo_push_token = "agent:{slug}", searchable_text = display_name)
    //   2. INSERT INTO agents (member_id, display_name, role, status = 'draft')
    //   3. INSERT INTO role-specific config table
    // The expo_push_token "agent:{slug}" convention prevents collision with real users.
    async fn list_agents(request: ListAgentsRequest) -> Result<Vec<AgentResponse>, HandlerError>;
    async fn get_agent(request: GetAgentRequest) -> Result<AgentDetailResponse, HandlerError>;
    async fn create_agent(request: CreateAgentRequest) -> Result<AgentResponse, HandlerError>;
    async fn update_agent(request: UpdateAgentRequest) -> Result<AgentResponse, HandlerError>;
    async fn set_agent_status(request: SetAgentStatusRequest) -> Result<AgentResponse, HandlerError>;

    // Curator config
    async fn update_curator_config(request: UpdateCuratorConfigRequest) -> Result<CuratorConfigResponse, HandlerError>;

    // CRUD - Search Queries (curator)
    async fn list_search_queries(request: AgentIdRequest) -> Result<Vec<AgentSearchQueryResponse>, HandlerError>;
    async fn create_search_query(request: CreateSearchQueryRequest) -> Result<AgentSearchQueryResponse, HandlerError>;
    async fn update_search_query(request: UpdateSearchQueryRequest) -> Result<AgentSearchQueryResponse, HandlerError>;
    async fn delete_search_query(request: DeleteRequest) -> Result<EmptyResponse, HandlerError>;

    // CRUD - Filter Rules (curator)
    async fn list_filter_rules(request: AgentIdRequest) -> Result<Vec<AgentFilterRuleResponse>, HandlerError>;
    async fn create_filter_rule(request: CreateFilterRuleRequest) -> Result<AgentFilterRuleResponse, HandlerError>;
    async fn update_filter_rule(request: UpdateFilterRuleRequest) -> Result<AgentFilterRuleResponse, HandlerError>;
    async fn delete_filter_rule(request: DeleteRequest) -> Result<EmptyResponse, HandlerError>;

    // Required Tag Kinds (curator)
    async fn set_required_tag_kinds(request: SetRequiredTagKindsRequest) -> Result<Vec<TagKindResponse>, HandlerError>;

    // Pipeline (curator)
    async fn run_agent_step(request: RunAgentStepRequest) -> Result<AgentRunResponse, HandlerError>;
    async fn run_scheduled_step(request: ScheduledStepRequest) -> Result<EmptyResponse, HandlerError>;

    // Runs
    async fn list_runs(request: ListRunsRequest) -> Result<Vec<AgentRunResponse>, HandlerError>;
    async fn get_run(request: GetRunRequest) -> Result<AgentRunDetailResponse, HandlerError>;
}
```

**Scheduling pattern** — self-scheduling via `send_after()`:

```rust
async fn run_scheduled_step(&self, ctx: Context<'_>, request: ScheduledStepRequest) -> Result<EmptyResponse, HandlerError> {
    let agent = Agent::find_by_id(request.agent_id, &self.deps.db_pool).await?;

    if agent.status != "active" || agent.role != "curator" {
        return Ok(EmptyResponse {}); // Don't reschedule if paused/draft/wrong role
    }

    let curator = AgentCuratorConfig::find_by_agent(agent.id, &self.deps.db_pool).await?;
    let step = &request.step; // "discover" or "monitor"

    // Schedule next run BEFORE executing (survives failures)
    // Follows existing pattern from discovery.rs:562
    let interval = match step.as_str() {
        "discover" => parse_schedule(&curator.schedule_discover),
        "monitor" => parse_schedule(&curator.schedule_monitor),
        _ => None,
    };
    if let Some(duration) = interval {
        ctx.service_client::<AgentsServiceClient>()
            .run_scheduled_step(ScheduledStepRequest { agent_id: agent.id, step: step.clone() })
            .send_after(duration);
    }

    // Run the step (failure doesn't break the schedule chain)
    let result = ctx.run(|| async {
        self.run_step_internal(&agent, &curator, step).await.map_err(Into::into)
    }).await;

    if let Err(e) = result {
        tracing::warn!(agent_id = %agent.id, step = %step, error = %e, "Scheduled step failed");
    }

    Ok(EmptyResponse {})
}
```

**Re-activation:** When `set_agent_status` transitions a curator from `paused` → `active`, kick off schedule chains:

```rust
async fn set_agent_status(&self, ctx: Context<'_>, request: SetAgentStatusRequest) -> Result<AgentResponse, HandlerError> {
    let agent = Agent::set_status(request.id, &request.status, &self.deps.db_pool).await?;

    // Kick off schedule chains when activating a curator
    if request.status == "active" && agent.role == "curator" {
        let curator = AgentCuratorConfig::find_by_agent(agent.id, &self.deps.db_pool).await?;
        if let Some(duration) = curator.schedule_discover.as_ref().and_then(|s| parse_schedule(s)) {
            ctx.service_client::<AgentsServiceClient>()
                .run_scheduled_step(ScheduledStepRequest { agent_id: agent.id, step: "discover".into() })
                .send_after(duration);
        }
        if let Some(duration) = curator.schedule_monitor.as_ref().and_then(|s| parse_schedule(s)) {
            ctx.service_client::<AgentsServiceClient>()
                .run_scheduled_step(ScheduledStepRequest { agent_id: agent.id, step: "monitor".into() })
                .send_after(duration);
        }
    }

    Ok(AgentResponse::from(agent))
}
```

**Key decisions:**
- `schedule_discover` triggers: Discover → Extract → Enrich (full pipeline)
- `schedule_monitor` triggers: Monitor only (re-crawl + sync)
- Draft agents: can be manually triggered, no scheduled runs
- Paused agents: no manual or scheduled runs
- "Run Now" while in-progress: rejected (check for active run before starting)
- Pipeline operations only valid for `role = "curator"` — service validates this

**Files changed/created:**
- `packages/server/src/domains/agents/restate/services/agents.rs` — new (replaces any existing agent service)
- `packages/server/src/domains/agents/restate/mod.rs` — new
- `packages/server/src/bin/server.rs` — register `AgentsServiceImpl`

#### Phase 4: Admin UI

**Replace Discovery + Extraction with Agents in sidebar:**

Sidebar Sources group changes from:
```
Sources: Websites, Discovery, Extraction
```
to:
```
Sources: Websites, Agents
```

**Agent list page:** `/admin/agents/page.tsx`
- Table: display_name, role badge (assistant/curator), status badge (draft/active/paused), last run (curators), post count (curators)
- "Create Agent" button → choose role, then enter details
- Click row → agent detail page

**Agent detail page:** `/admin/agents/[id]/page.tsx`

Content adapts based on role:

**All roles — shared header:**
- Display name (editable)
- Role badge (read-only after creation)
- Status toggle (draft → active → paused)

**Assistant-specific tabs:**

**Tab: Configuration**
- Preamble (textarea, editable)
- Config name

**Curator-specific tabs:**

**Tab: Overview**
- Purpose (textarea, editable)
- Audience roles (checkboxes: recipient, volunteer, donor, participant)
- Required tag kinds (multi-select from existing tag_kinds — e.g., service_language, service_area)
- Schedule config: two dropdowns (discover cadence, monitor cadence) with presets: "Every 6 hours", "Daily", "Every 3 days", "Weekly", "Manual only"

**Tab: Search Queries**
- List of queries with query_text, is_active toggle
- Add/edit/delete
- `{location}` helper text

**Tab: Filter Rules**
- List of plain-text rules with is_active toggle
- Add/edit/delete

**Tab: Runs**
- Run history table: step, trigger, status, stats summary, started_at, duration
- "Run Now" buttons for each step (Discover, Extract, Enrich, Monitor)
- Buttons disabled when run in-progress

**Tab: Websites**
- Websites discovered by this agent (via `agent_websites`)
- Links to website detail page

**Tab: Posts**
- Posts created by this agent (filtered by `agent_id`)
- "Missing tags" indicator on posts lacking required tag kinds
- Links to post detail page

**Posts page enhancement:**
- Add agent filter dropdown to existing posts list
- Requires backend change: add optional `agent_id` field to `ListPostsRequest` in `packages/server/src/domains/posts/restate/services/posts.rs` and update `Post::find_paginated` in `packages/server/src/domains/posts/models/post.rs` to filter by agent_id

**Website detail page:**
- Replace Discovery sources tab (currently calls `Discovery.get_website_sources`) with Agents tab showing which agents have linked this website (via `agent_websites`)

**Middleware:**
- Add `/admin/agents` to protected routes in `packages/web/middleware.ts`
- Remove `/admin/extraction` from protected routes

**Files changed/created:**
- `packages/web/app/admin/(app)/agents/page.tsx` — new (agent list)
- `packages/web/app/admin/(app)/agents/[id]/page.tsx` — new (agent detail, role-adaptive)
- `packages/web/components/admin/AdminSidebar.tsx` — replace Discovery+Extraction with Agents
- `packages/web/lib/restate/types.ts` — add Agent types (base + assistant config + curator config)
- `packages/web/app/admin/(app)/posts/[id]/page.tsx` — show agent name
- `packages/web/app/admin/(app)/websites/[id]/page.tsx` — replace Discovery tab with Agents tab (remove `Discovery.get_website_sources` call)
- `packages/web/middleware.ts` — add `/admin/agents`, remove `/admin/extraction`
- `packages/server/src/domains/posts/restate/services/posts.rs` — add `agent_id` to `ListPostsRequest`
- `packages/server/src/domains/posts/models/post.rs` — add `agent_id` filter to `find_paginated`
- `packages/web/app/admin/(app)/discovery/page.tsx` — delete

#### Phase 5: Cleanup

**Migration: `000129_drop_discovery_tables.sql`** (runs LAST, after all agents code is live)

```sql
DROP TABLE IF EXISTS discovery_run_results;
DROP TABLE IF EXISTS discovery_runs;
DROP TABLE IF EXISTS discovery_filter_rules;
DROP TABLE IF EXISTS discovery_queries;
```

**Why last:** Dropping tables in an earlier phase would break incremental rollout. The agents domain must be fully functional before removing discovery infrastructure.

- [x] Remove `domains/discovery/` entirely (models, activities, restate service)
- [x] Remove Discovery admin page
- [x] Remove Extraction admin page (extraction is now per-agent)
- [ ] Remove hardcoded `POST_SEARCH_QUERY` constant from `crawling/activities/post_extraction.rs` (deferred — still used by crawling domain)
- [x] Remove Discovery service registration from `server.rs`
- [x] Clean up TypeScript types for discovery
- [ ] Remove stale indexes from old agents table (`idx_agents_enabled`, `idx_agents_due`) if not already handled by migration (already handled by 000126)

**Files deleted:**
- `packages/server/src/domains/discovery/` — entire directory
- `packages/web/app/admin/(app)/discovery/` — entire directory
- `packages/web/app/admin/(app)/extraction/` — entire directory (if applicable)

**Files changed:**
- `packages/server/src/domains/mod.rs` — remove `pub mod discovery`
- `packages/server/src/bin/server.rs` — remove Discovery service binding
- `packages/web/lib/restate/types.ts` — remove Discovery types
- `packages/web/app/admin/(app)/websites/[id]/page.tsx` — verify Discovery tab fully replaced by Agents tab (Phase 4)

## Acceptance Criteria

### Functional Requirements

- [ ] Agents have roles: `assistant` and `curator`
- [ ] All agents have member identities (member_id on base table, synthetic member provisioned on create)
- [ ] Existing chatbot agents migrated to `role = 'assistant'` with config in `agent_assistant_configs`
- [ ] Admin can create a curator agent with display_name and purpose
- [ ] Admin can add/edit/delete search queries per curator
- [ ] Admin can add/edit/delete filter rules per curator
- [ ] Admin can set schedule (discover + monitor cadence) per curator
- [ ] Admin can configure required tag kinds per curator (multi-select from tag_kinds)
- [ ] Admin can set agent status (draft/active/paused) for any role
- [ ] Admin can manually trigger any pipeline step for a curator
- [ ] Discover step: searches web, applies filters, creates/links websites
- [ ] Extract step: uses curator's purpose in extraction prompts, creates posts with agent_id
- [ ] Required tag kinds are injected into extraction prompts
- [ ] Enrich step: investigates agent's posts for missing contact/location AND missing required tags
- [ ] Admin UI shows "missing tags" indicator on posts lacking required tag kinds (derived at query time, no new post status)
- [ ] Monitor step: re-crawls agent's websites, creates sync proposals scoped to agent's posts
- [ ] Scheduled runs fire based on curator's cadence settings
- [ ] Schedules reschedule before execution (survive failures)
- [ ] Re-activating a paused curator kicks off schedule chains
- [ ] Paused agents don't run (manual or scheduled)
- [ ] Draft agents can be manually triggered but don't run on schedule
- [ ] Multiple curators can independently discover and extract from the same website
- [ ] Run history shows stats per step
- [ ] Posts page can be filtered by agent
- [ ] Agent list page shows both assistants and curators
- [ ] Agent detail page adapts UI based on role
- [ ] Discovery admin page removed
- [ ] Middleware protects `/admin/agents`, removes `/admin/extraction`

### Quality Gates

- [ ] All models use `sqlx::query_as::<_, Type>` (no macros)
- [ ] All SQL queries in models, not activities
- [ ] No JSONB columns (normalized stats table)
- [ ] Existing migrations unmodified
- [ ] Restate types use `impl_restate_serde!`
- [ ] Pipeline operations validate `role = "curator"` before executing

## Key Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Unified agent model | Base table + role-specific config tables | Chatbot agents and curator agents are both autonomous entities — same concept, different roles |
| Role names | `assistant` (chat) and `curator` (pipeline) | Assistant helps people, curator manages content. Clear, distinct. |
| Member identity | All agents get member_id on base table | Agents are treated like normal users in the system |
| Agent-website relationship | Explicit `agent_websites` join table | Multiple curators can share websites; need to track who discovered what |
| Post ownership | Nullable `agent_id` FK on posts | Existing posts predate agents; new posts always have agent_id |
| Schedule semantics | discover = full pipeline, monitor = re-crawl+sync | Admins shouldn't have to manually chain steps after discovery |
| Schedule resilience | Reschedule before execution | Follows existing discovery pattern; failures don't break schedule chain |
| Stats storage | Normalized `agent_run_stats` (key-value) | Avoids JSONB per CLAUDE.md, different steps have different metrics |
| Monitor behavior | Creates sync proposals (human-in-the-loop) | Consistent with existing system, prevents accidental data loss |
| Schedule format | Interval in hours, dropdown presets in UI | Simple, covers all use cases without cron complexity |
| Required tag kinds | Join table to tag_kinds, no free-text | Leverages existing tag system, programmatically enforceable |
| Post completeness | Derived at query time from required tags | No new post status needed; existing constraint unchanged |
| Discovery cleanup | Drop tables last (Phase 5) | Agents must be fully live before removing discovery infrastructure |

## References & Research

### Internal References
- Brainstorm: `docs/brainstorms/2026-02-08-agents-brainstorm.md`
- Existing agents (chatbot): `packages/server/src/domains/agents/models/agent.rs`
- Existing discovery: `packages/server/src/domains/discovery/` (being replaced)
- Extraction pipeline: `packages/server/src/domains/crawling/activities/post_extraction.rs`
- Post creation: `packages/server/src/domains/posts/activities/create_post.rs`
- Tag instructions: `packages/server/src/domains/tag/models/tag_kind_config.rs:114-157`
- Admin sidebar: `packages/web/components/admin/AdminSidebar.tsx`
- Restate client: `packages/web/lib/restate/client.ts`
- Scheduling pattern: `packages/server/src/domains/discovery/restate/services/discovery.rs:562-565`
- Server registration: `packages/server/src/bin/server.rs:233-268`
- Post status constraint: `packages/server/migrations/000032_refactor_needs_to_listings.sql:61`
- Existing agent indexes: `packages/server/migrations/000052_migrate_search_topics_to_agents.sql:14-15`
- Agent config index: `packages/server/migrations/000112_add_agent_config_name.sql:5`
- Website detail discovery tab: `packages/web/app/admin/(app)/websites/[id]/page.tsx:86,552`
- Posts list request: `packages/server/src/domains/posts/restate/services/posts.rs:29`
- Middleware routes: `packages/web/middleware.ts:7-14`
- Filter evaluation: `packages/server/src/domains/discovery/activities/evaluate_filter.rs`

### CLAUDE.md Conventions
- `sqlx::query_as::<_, Type>` (never macro)
- Never modify existing migrations
- No JSONB — normalize into tables
- SQL queries in models only
- Activities are pure functions taking `&ServerDeps`
