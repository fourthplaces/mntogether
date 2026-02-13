---
date: 2026-02-08
topic: agents
---

# Agents: Purpose-Driven Pipelines

## What We're Building

An **agent** is a configurable, schedulable pipeline that discovers websites, extracts posts, enriches them, and keeps them up to date. Each agent has a specific purpose (e.g., "find volunteer opportunities," "find local businesses to support," "find local events") that shapes every stage of its pipeline.

This replaces the current architecture where discovery is database-driven but extraction uses hardcoded prompts. Instead, the agent's purpose flows through the entire lifecycle.

## Why This Approach

**Problem:** Posts are being extracted that are unrelated to what we care about because extraction prompts are hardcoded and one-size-fits-all. A page about a bakery gets shoehorned into "community resources" framing or ignored entirely.

**Solution:** Each agent owns its full lifecycle — search, filter, extract, enrich, monitor — with its own purpose-specific instructions. The extraction prompt for a "local businesses" agent is fundamentally different from a "volunteer opportunities" agent.

**Approaches considered:**
- Making prompts configurable via admin UI (too narrow — only fixes extraction)
- General-purpose "task runner" system (too abstract — YAGNI)
- Agent-per-pipeline-step (chose agent-owns-everything since websites can be shared across agents naturally)

## Data Model

### Agent

| Field | Type | Purpose |
|-------|------|---------|
| id | UUID | Primary key |
| name | TEXT | Display name ("Volunteer Opportunities") |
| purpose | TEXT | Free-text instructions injected into extraction prompts |
| audience_roles | TEXT[] | Which roles apply (recipient, volunteer, donor, participant) |
| schedule_discover | TEXT | Cron or interval for discovery runs |
| schedule_monitor | TEXT | Cron or interval for monitoring/re-crawl |
| status | TEXT | draft / active / paused |
| created_at | TIMESTAMPTZ | |
| updated_at | TIMESTAMPTZ | |

### Agent Search Queries

| Field | Type | Purpose |
|-------|------|---------|
| id | UUID | Primary key |
| agent_id | UUID FK | Owning agent |
| query_text | TEXT | Search query with {location} templating |
| is_active | BOOLEAN | Enable/disable |
| sort_order | INT | Execution order |

### Agent Filter Rules

| Field | Type | Purpose |
|-------|------|---------|
| id | UUID | Primary key |
| agent_id | UUID FK | Owning agent |
| rule_text | TEXT | AI-evaluated inclusion/exclusion rule |
| is_active | BOOLEAN | Enable/disable |
| sort_order | INT | Execution order |

### Agent Runs

| Field | Type | Purpose |
|-------|------|---------|
| id | UUID | Primary key |
| agent_id | UUID FK | Which agent ran |
| step | TEXT | discover / extract / enrich / monitor |
| trigger_type | TEXT | manual / scheduled |
| stats | JSONB | Step-specific metrics (queries run, posts created, etc.) |
| started_at | TIMESTAMPTZ | |
| completed_at | TIMESTAMPTZ | |

### Post Ownership

Posts gain an `agent_id` foreign key. Multiple agents can create posts from the same website — each agent extracts what's relevant to its purpose.

## Pipeline Steps

### 1. Discover
- Run agent's search queries via Tavily
- Apply agent's filter rules (AI-evaluated)
- Create/link websites (websites are shared infrastructure)
- Log results in agent run

### 2. Extract
- For each website linked to this agent, run extraction
- Agent's `purpose` text is injected into the extraction prompt template
- Agent's `audience_roles` control audience splitting behavior
- Tag classification uses existing dynamic tag_kinds system
- Posts created with `agent_id` set

### 3. Enrich
- Investigate posts for missing contact info, location, hours
- Uses existing agentic investigation (fetch_page + web_search tools)
- Scoped to posts owned by this agent

### 4. Monitor
- Re-crawl websites linked to this agent
- Run LLM sync to update/insert/delete posts
- Scoped to this agent's posts only (won't touch other agents' posts)

## Prompt Architecture

**Hybrid approach:** Free-text purpose + structured fields.

The agent's `purpose` field is the "soul" — it gets injected into a prompt template that handles the mechanical parts (output format, deduplication rules, confidence scoring, etc.).

```
Template: "You are extracting {purpose} from website content. {audience_instructions} {tag_instructions} {format_instructions}"
```

Admins write the purpose ("volunteer opportunities at nonprofits and community organizations"). The system handles everything else.

## Admin UI

- **Agents list** — name, status badge, last run time, post count
- **Agent detail page:**
  - Purpose editor (free-text)
  - Audience role toggles
  - Search queries (CRUD)
  - Filter rules (CRUD)
  - Schedule configuration
  - Run history with stats
  - "Run Now" button per step (discover / extract / enrich / monitor)
- **Posts view** — filterable by agent

## Key Decisions

- **Clean slate**: Old `discovery_*` tables get dropped. No backwards compatibility needed.
- **Agent owns everything**: Search queries, filter rules, extraction instructions, and posts all belong to an agent.
- **Websites are shared**: Multiple agents can discover and extract from the same website independently.
- **Hybrid prompt config**: Free-text purpose + structured mechanical fields. No need for admins to write full prompts.
- **Per-agent scheduling**: Each agent has its own discover and monitor cadence.
- **Posts belong to agents**: `agent_id` FK on posts table.

## Open Questions

- Should there be global filter rules that apply to ALL agents (e.g., "skip .gov domains"), or should each agent fully own its filters?
- How granular should scheduling be? Per-step (discover weekly, monitor daily) or one cadence per agent?
- Should agent runs be visible cross-agent (unified "runs" view) or only within each agent's detail page?
- How should we handle the join table between agents and websites? Implicit (via posts) or explicit (agent_websites table)?

## Next Steps

→ `/workflows:plan` for implementation details
