# Curator Pipeline

The curator is the AI system that turns crawled web pages into actionable community posts. It reads everything known about an organization — website content, social media, existing posts, notes — and proposes create, update, merge, archive, or flag actions. Every proposal goes through safety review and lands in a sync batch for human approval before anything is published.

This pipeline exists because the platform serves immigrant communities in Minnesota. Getting eligibility restrictions wrong isn't a UX bug — it's a safety risk.

## Architecture Overview

```
                                    TRIGGERS
                    ┌─────────────────┼─────────────────┐
                    │                 │                 │
              Crawl Complete    Scheduled (15m)    Admin Manual
                    │                 │                 │
                    └────────┬────────┘                 │
                             │                          │
                    ┌────────▼──────────────────────────▼─┐
                    │        CurateOrgWorkflow (Restate)   │
                    │                                      │
                    │  ┌──────────────────────────────┐    │
                    │  │ Phase 1: Load Org + Sources   │    │
                    │  └──────────┬───────────────────┘    │
                    │             │                         │
                    │  ┌──────────▼───────────────────┐    │
                    │  │ Phase 2: Fetch Crawled Pages  │    │
                    │  │    (Extraction Service)       │    │
                    │  └──────────┬───────────────────┘    │
                    │             │                         │
                    │  ┌──────────▼───────────────────┐    │
                    │  │ Phase 3: Extract Page Briefs  │◄── LLM (GPT-5-mini)
                    │  │    MAP: 1 call per page       │    │  memoized 30 days
                    │  │    10 concurrent              │    │
                    │  └──────────┬───────────────────┘    │
                    │             │                         │
                    │  ┌──────────▼───────────────────┐    │
                    │  │ Phase 4: Compile Org Document │    │  deterministic
                    │  │    (briefs + posts + notes)   │    │
                    │  └──────────┬───────────────────┘    │
                    │             │                         │
                    │  ┌──────────▼───────────────────┐    │
                    │  │ Phase 5: Curator Reasoning    │◄── LLM (GPT-5-mini)
                    │  │    REDUCE: 1 call total       │    │  single call
                    │  └──────────┬───────────────────┘    │
                    │             │                         │
                    │  ┌──────────▼───────────────────┐    │
                    │  │ Phase 5.5: Writer Rewrite     │◄── LLM (Claude Sonnet)
                    │  │    1 call per post, parallel   │    │  fallback: GPT-5-mini
                    │  └──────────┬───────────────────┘    │
                    │             │                         │
                    │  ┌──────────▼───────────────────┐    │
                    │  │ Phase 5.7: Safety Review      │◄── LLM (GPT-5-mini)
                    │  │    up to 3 review loops       │    │  iterative
                    │  └──────────┬───────────────────┘    │
                    │             │                         │
                    │  ┌──────────▼───────────────────┐    │
                    │  │ Phase 6: Stage Proposals      │    │  DB writes
                    │  │    (drafts + sync batch)      │    │
                    │  └──────────┬───────────────────┘    │
                    │             │                         │
                    │  ┌──────────▼───────────────────┐    │
                    │  │ Phase 7: Update Timestamps    │    │
                    │  └──────────────────────────────┘    │
                    │                                      │
                    └──────────────────────────────────────┘
                             │
                    ┌────────▼────────┐
                    │   SyncBatch     │
                    │   (pending)     │──► Admin reviews in UI
                    │   SyncProposal  │──► Approve / Reject / Refine
                    └─────────────────┘
```

## Triggers

The curator runs for a single organization at a time. Three things can start it:

**1. After a website crawl completes**
`crawling/restate/workflows/crawl_website.rs:91-96` — When `CrawlWebsiteWorkflow` finishes extracting pages, it fires off `CurateOrgWorkflow` for the website's parent organization.

**2. Scheduled extraction (every 15 minutes)**
`organization/restate/services/organizations.rs:823-829` — `run_scheduled_extraction` finds orgs that haven't been extracted recently and triggers both extraction and curator workflows.

**3. Manual admin trigger**
`organization/restate/services/organizations.rs:870-892` — An admin calls `run_curator` with an organization ID. Generates a unique workflow key with a timestamp so it always runs fresh.

All three call the same workflow:
```rust
CurateOrgRequest { organization_id: Uuid }
```

## Pipeline Phases

### Phase 1: Load Organization + Sources

**File:** `curator/restate/workflows/curate_org.rs:64-91`

Loads the `Organization` record and all `Source` records linked to it. Each source represents a website or social media profile. Converts sources to their `site_url` (the domain or profile URL). Exits early with `status: "no_sources"` if none exist.

### Phase 2: Fetch Crawled Pages

**File:** `curator/restate/workflows/curate_org.rs:93-116`

Calls the Extraction Service (a separate microservice that manages a vector DB of crawled content) to retrieve all `CachedPage` records for each source URL. Each page has a `url` and `content` field. Exits early with `status: "no_pages"` if nothing has been crawled yet.

```rust
let extraction = self.deps.extraction.as_ref()?;
for url in &source_urls {
    if let Ok(site_pages) = extraction.get_pages_for_site(url).await {
        pages.extend(site_pages);
    }
}
```

### Phase 3: Extract Page Briefs (Map Step)

**File:** `curator/activities/brief_extraction.rs`

For each crawled page, an LLM extracts structured data into a `PageBriefExtraction`. This is the map step — one LLM call per page, up to 10 concurrent.

| Detail | Value |
|--------|-------|
| Model | GPT-5-mini |
| Concurrency | 10 pages at once |
| Memoization | 30-day TTL, keyed by system prompt + page content |
| Input | Page content (truncated to 50KB) |
| Min content | 100 chars (shorter pages are skipped) |

**What gets extracted from each page:**

```rust
PageBriefExtraction {
    summary: String,                   // 2-3 sentence overview
    locations: Vec<String>,            // Full physical addresses
    calls_to_action: Vec<String>,      // Donation drives, volunteer asks, supply needs
    critical_info: Option<String>,     // Hours, eligibility, deadlines, capacity
    services: Vec<String>,             // Programs offered by name
    contacts: Vec<BriefContact>,       // Phone, email, website, booking URLs
    schedules: Vec<BriefSchedule>,     // Operating hours, events, recurring patterns
    languages_mentioned: Vec<String>,  // Spanish, Somali, Karen, Hmong...
    populations_mentioned: Vec<String>,// Refugees, immigrants, seniors, youth...
    capacity_info: Option<String>,     // "accepting", "waitlist", "at capacity"
}
```

The extraction prompt (lines 8-59) emphasizes separating eligibility restrictions by audience/activity. If one program requires citizenship but another doesn't, the brief must capture that distinction — not lump them together.

### Phase 4: Compile Org Document (Deterministic)

**File:** `curator/activities/org_document.rs`

No LLM involved. Takes all page briefs, existing posts (with contacts, schedules, tags), and existing notes, and formats them into a single markdown document with a 200,000 character budget (~50k tokens).

**Document structure:**
```
# Organization: [Name]
## Website Content          ← website briefs (highest priority)
## Social Media             ← social media briefs
## Existing Posts in System ← current posts with [POST-uuid] IDs
## Existing Notes           ← active notes with [NOTE-uuid] IDs
```

Priority ordering ensures website content gets full space, then social, then existing state. The budget prevents blowing up context windows.

**Database queries used:**
- `Post::find_by_organization_id()` — all posts for the org
- `Contact::find_by_entity("post", id)` — contacts per post
- `Schedule::find_for_post(id)` — schedules per post
- `Tag::find_for_post(id)` — tags per post
- `Note::find_active_for_entity("post", id)` — notes per post

### Phase 5: Curator Reasoning (Reduce Step)

**File:** `curator/activities/curator.rs`

The core intelligence. One LLM call reads the entire org document and proposes all actions at once.

| Detail | Value |
|--------|-------|
| Model | GPT-5-mini |
| Input | Full org document (all briefs + posts + notes) |
| Output | `CuratorResponse` with `Vec<CuratorAction>` |
| System prompt | 162 lines (curator.rs:6-162) |

**Available action types:**

| Action | When to use |
|--------|-------------|
| `create_post` | New service/event that helps people in danger or immediate need |
| `update_post` | Existing post needs changes (references `POST-{uuid}`) |
| `add_note` | Context that doesn't warrant a full update |
| `merge_posts` | Two+ posts describe the same thing |
| `archive_post` | Post is stale or no longer relevant |
| `flag_contradiction` | Sources disagree about something important |

**Key reasoning rules in the system prompt:**

- **Social media overrides website.** Website says "open Monday-Friday" but Instagram says "closed this week" → they're closed. Social is more current.
- **Crisis-response litmus test.** Would this help someone who could get deported, evicted, or go hungry? If yes, post it. If it's a nice community program that would exist regardless of the crisis, skip it.
- **One post = one action.** Volunteering, donating money, dropping off supplies, and getting help are all separate posts. Different eligibility = different posts.
- **Deduplication.** Check existing feed before creating. Two posts about "food" aren't duplicates if one is "drop off groceries" (giving) and the other is "get groceries delivered" (receiving).
- **Never create posts for paused/closed services.** If social media says it's paused, don't create a post. Archive existing posts about it instead.

**Validation:** Every action must have at least one `source_url` backing it (except merges, which reference existing posts).

### Phase 5.5: Writer Rewrite (Parallel)

**File:** `curator/activities/writer.rs`

The curator's output is accurate but robotic. The writer makes it sound like a neighbor wrote it.

| Detail | Value |
|--------|-------|
| Model | Claude Sonnet (preferred), GPT-5-mini (fallback) |
| Scope | `create_post` and `update_post` actions only |
| Concurrency | All rewrites in parallel via `join_all` |
| Input per post | Draft action + org document excerpt (20KB cap) + existing feed for angle dedup |

**What the writer produces:**

```rust
PostCopy {
    title: String,   // 5-10 words, action-first ("Talk to an Immigration Lawyer for Free")
    summary: String, // 2-3 sentences, 250 char hard max
    text: String,    // 150-300 words of flowing markdown prose
}
```

**Writer rules (248-line system prompt):**

- Write like you're texting a friend who asked "how can I help?"
- No process documentation. "Registrations entered in the new system are confirmed and final" becomes "Pick a shift and show up."
- Stay in your lane. A donate post never mentions dropping off groceries.
- Never fabricate stories, quotes, or testimonials.
- Never use the org name in the opening.
- End with a friction reducer (parking, what to expect, who to text) — never with a contact block.
- Never repeat angles from the existing feed.
- Never soften or omit eligibility restrictions.

If the writer fails for a particular post, the curator's draft copy is kept as-is.

### Phase 5.7: Safety Review (Iterative)

**File:** `curator/activities/safety_review.rs`

Reviews only `create_post` and `update_post` actions. Checks whether each post omits eligibility restrictions that exist in the source material.

| Detail | Value |
|--------|-------|
| Model | GPT-5-mini |
| Max loops | 3 (review → fix → re-review) |
| Input per review | Posts with their matching source briefs |

**What gets checked:**

1. Citizenship or residency requirements
2. ID requirements (driver's license, proof of status)
3. Age restrictions (18+, adults only)
4. Geographic restrictions (service area limits)
5. Registration requirements (source says "register" but post says "just show up")
6. False safety claims ("no questions asked" when source lists restrictions)

**Review loop:**

```
For each attempt (max 3):
  1. Send all unreviewed posts to LLM with their source briefs
  2. For each post, LLM returns:
     - "safe"    → done, move on
     - "fix"     → LLM provides corrected description, re-review next loop
     - "blocked" → post is removed from the batch entirely
  3. If no fixes this round, stop early
  4. After 3 failed fix attempts for one post, it's blocked
```

Posts that pass are done. Only fixed posts get re-reviewed. Blocked posts are removed from the action list before staging.

### Phase 6: Stage Actions as Proposals

**File:** `curator/activities/stage_actions.rs`

Converts curator actions into database entities and sync proposals for human review.

**Before staging:** Expires any stale pending curator batches for this org. Each expired batch has its proposals rejected and draft entities (posts, notes) cleaned up. This prevents orphaned drafts from accumulating.

**Per action type:**

| Action | What gets created |
|--------|-------------------|
| `create_post` | Draft `Post` (status=draft, submission_type=agent) + `PostSource` links + `Location`/`Locationable` + `Contact` records + `Schedule` records + schedule `Note` + `Tag`/`Taggable` + embedding + `SyncProposal` (operation=insert) |
| `update_post` | Revision `Post` (with `revision_of_post_id` linking to original) + `SyncProposal` (operation=update) |
| `add_note` | Draft `Note` + optional `Noteable` link to target post + `SyncProposal` (operation=insert) |
| `merge_posts` | `SyncProposal` (operation=merge) + `SyncProposalMergeSource` records |
| `archive_post` | `SyncProposal` (operation=delete) |
| `flag_contradiction` | Draft `Note` (severity=urgent) + `SyncProposal` (operation=insert) |

**Schedule creation supports three modes:**

1. **Operating hours:** `day_of_week` + `opens_at` + `closes_at` (both required — entries with only `opens_at` are rejected as likely generic org hours)
2. **Recurring:** `frequency` or `rrule` + `day_of_week` + times
3. **One-off event:** `date` + `start_time`/`end_time` or `is_all_day`

Everything is grouped into a `SyncBatch` (resource_type=`"curator"`) with the curator's org summary as description.

### Phase 7: Update Timestamps

**File:** `curator/restate/workflows/curate_org.rs:280-283`

Updates `Organization.last_extracted_at` to signal that this org was recently curated. Used by the scheduled extraction trigger to determine which orgs need attention.

## Proposal Refinement (Secondary Workflow)

**File:** `curator/restate/workflows/refine_proposal.rs` + `curator/activities/refine_proposal.rs`

After proposals are staged, admins can leave feedback comments. The `RefineProposalWorkflow` takes an admin comment and revises the draft entity.

| Detail | Value |
|--------|-------|
| Model | GPT-5-mini |
| Max revisions per proposal | 3 |
| Input | Original proposal + comment history + latest comment |

The admin has final say — if their feedback contradicts source material, the LLM notes the contradiction but makes the requested change anyway. After max revisions, new comments are saved but no more AI revisions happen.

## LLM Calls Summary

| # | Phase | Function | Model | Calls | Memoized? |
|---|-------|----------|-------|-------|-----------|
| 1 | Brief extraction | `extract_page_brief()` | GPT-5-mini | 1 per page (10 concurrent) | 30-day TTL |
| 2 | Curator reasoning | `run_curator()` | GPT-5-mini | 1 per org | No |
| 3 | Writer rewrite | `rewrite_post_copy()` | Claude Sonnet / GPT-5-mini | 1 per post (parallel) | No |
| 4 | Safety review | `review_and_fix_actions()` | GPT-5-mini | 1-3 per batch (iterative) | No |
| 5 | Proposal refinement | `refine_proposal_from_comment()` | GPT-5-mini | 1 per admin comment (max 3) | No |

## Database Tables

| Table | Curator usage |
|-------|---------------|
| `organizations` | Load org, update `last_extracted_at` |
| `sources` | Load sources for org, get site URLs |
| `posts` | Find existing posts, create drafts (status=draft, submission_type=agent) |
| `post_sources` | Link draft posts to their source records |
| `contacts` | Find contacts for existing posts, create contacts for drafts |
| `schedules` | Find schedules for existing posts, create schedules for drafts |
| `tags` / `taggables` | Find tags for existing posts, find/create + link tags for drafts |
| `locations` / `locationables` | Create locations for draft posts |
| `notes` / `noteables` | Find existing notes, create draft notes + schedule notes |
| `sync_batches` | Create batch (resource_type=curator), expire stale batches |
| `sync_proposals` | Create one proposal per action, track revision count |
| `sync_proposal_merge_sources` | Track merge source posts |
| `proposal_comments` | Store admin feedback + revision history |

## Safety Architecture

The entire pipeline is built around one invariant: **never omit eligibility restrictions.**

This is enforced at four layers:

1. **Brief extraction prompt** (brief_extraction.rs:24-29) — Instructs the LLM to separate eligibility restrictions by audience/activity. "Volunteers: must be 18+, US citizens. Recipients: open to all, no ID required."

2. **Curator system prompt** (curator.rs:150-156) — "NEVER soften, omit, or generalize eligibility restrictions." Different eligibility means different posts.

3. **Writer system prompt** (writer.rs:243-247) — "Never soften or omit eligibility restrictions." Do NOT write "no questions asked" or "no paperwork" when restrictions exist.

4. **Safety review loop** (safety_review.rs:83-203) — Compares each post against its source material. Checks for citizenship requirements, ID requirements, age restrictions, geographic restrictions, registration requirements, and false safety claims. Can auto-fix minor omissions. Blocks posts that can't be fixed after 3 attempts.

## File Reference

| Component | Path (relative to `packages/server/src/domains/curator/`) |
|-----------|-----------------------------------------------------------|
| **Main workflow** | `restate/workflows/curate_org.rs` |
| **Refinement workflow** | `restate/workflows/refine_proposal.rs` |
| **Brief extraction** | `activities/brief_extraction.rs` |
| **Org document compiler** | `activities/org_document.rs` |
| **Curator reasoning** | `activities/curator.rs` |
| **Writer rewrite** | `activities/writer.rs` |
| **Safety review** | `activities/safety_review.rs` |
| **Stage actions** | `activities/stage_actions.rs` |
| **Proposal refinement** | `activities/refine_proposal.rs` |
| **Note proposal handler** | `activities/note_proposal_handler.rs` |
| **Type definitions** | `models/types.rs` |

| Trigger | Path (relative to `packages/server/src/`) |
|---------|-------------------------------------------|
| Post-crawl trigger | `domains/crawling/restate/workflows/crawl_website.rs:91-96` |
| Scheduled extraction | `domains/organization/restate/services/organizations.rs:823-829` |
| Manual admin trigger | `domains/organization/restate/services/organizations.rs:870-892` |
| Server registration | `bin/server.rs:290-291` |
