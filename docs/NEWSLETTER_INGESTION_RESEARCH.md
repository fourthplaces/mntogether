---
title: "Newsletter Ingestion Research Summary"
type: research
date: 2026-02-13
---

# Newsletter Ingestion: Institutional Learnings & Architecture Research

## Executive Summary

Adding newsletter ingestion as a third source type involves leveraging existing patterns from the unified sources refactor (class table inheritance), the extraction pipeline (Ingestor trait), and the Restate workflow architecture. This research document consolidates existing architectural decisions, extracted patterns, and gotchas to prevent repeated mistakes during implementation.

---

## 1. Source Type Architecture (Class Table Inheritance Pattern)

### Current State

The unified sources refactor (migration `000149_create_unified_sources.sql`) established a **class table inheritance pattern** that you should extend for newsletter_sources:

```sql
sources (parent)
├── source_type TEXT  -- 'website', 'instagram', 'facebook', 'tiktok'
├── organization_id UUID FK
├── status TEXT  -- 'pending_review', 'approved', 'rejected', 'suspended'
├── active BOOLEAN
├── scrape_frequency_hours INT
├── last_scraped_at TIMESTAMPTZ
├── submitted_by, submitter_type, submission_context
├── reviewed_by, reviewed_at, rejection_reason
├── created_at, updated_at

website_sources (1:1 child)
├── source_id UUID UNIQUE FK → sources
├── domain TEXT UNIQUE
├── max_crawl_depth INT
├── crawl_rate_limit_seconds INT
├── is_trusted BOOLEAN

social_sources (1:1 child)
├── source_id UUID UNIQUE FK → sources
├── source_type TEXT  -- denormalized for UNIQUE constraint
├── handle TEXT
├── UNIQUE(source_type, handle)
```

### Pattern for Newsletter Sources

**Add `newsletter_sources` table** following the same pattern:

```sql
CREATE TABLE newsletter_sources (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    source_id UUID NOT NULL UNIQUE REFERENCES sources(id) ON DELETE CASCADE,

    -- Email ingestion
    ingest_email TEXT NOT NULL UNIQUE,  -- {uuid}@ingest.mntogether.org

    -- Subscription flow
    subscription_status TEXT NOT NULL DEFAULT 'subscribing',
    -- Values: 'subscribing', 'pending_confirmation', 'active', 'inactive'

    -- Form submission
    detected_signup_url TEXT,  -- where the form was found
    confirmation_url TEXT,    -- extracted from confirmation email

    -- Ingest metadata
    last_email_received_at TIMESTAMPTZ,
    email_count INT DEFAULT 0  -- for analytics
);

-- Indexes
CREATE INDEX idx_newsletter_sources_source_id ON newsletter_sources(source_id);
CREATE INDEX idx_newsletter_sources_ingest_email ON newsletter_sources(ingest_email);
CREATE INDEX idx_newsletter_sources_subscription_status ON newsletter_sources(subscription_status);
```

### Key Decisions Extracted

1. **Class table inheritance over single-table polymorphism**: Keeps type-specific fields normalized, avoids nullable sprawl
2. **Denormalize `source_type` on child table when needed for constraints**: `social_sources` does this for the `UNIQUE(source_type, handle)` constraint
3. **Foreign key to parent on child table as UNIQUE**: Enforces 1:1 relationship and enables efficient lookups
4. **Use ON DELETE CASCADE**: Child tables disappear when parent is deleted — clean cascade
5. **Add indexes on all lookups**: `source_id`, `ingest_email` (fast route lookup), `subscription_status` (query for pending confirmations)

### Migration Gotchas from Unified Sources

- **Order matters**: Create tables → migrate data → update FKs on dependent tables → recreate views → drop old tables
- **Views depend on tables**: Migration `000149` had to drop and recreate `domain_statistics` and `page_snapshot_details` views
- **Test FK constraints heavily**: The FK rework touched 7+ dependent tables; test each one

---

## 2. Extraction Pipeline Integration (Newsletter as "Just Pages")

### Current Architecture

The extraction pipeline is **source-agnostic** via the `Ingestor` trait:

**File**: `/packages/extraction/src/traits/ingestor.rs`

```rust
pub struct RawPage {
    pub url: String,           // identifier
    pub content: String,       // markdown/HTML/text
    pub title: Option<String>,
    pub content_type: Option<String>,
    pub fetched_at: DateTime<Utc>,
    pub metadata: HashMap<String, String>,  // extensible
}

#[async_trait]
pub trait Ingestor: Send + Sync {
    async fn discover(&self, config: &DiscoverConfig) -> CrawlResult<Vec<RawPage>>;
    async fn fetch_specific(&self, urls: &[String]) -> CrawlResult<Vec<RawPage>>;
    async fn fetch_one(&self, url: &str) -> CrawlResult<RawPage>;
    fn name(&self) -> &str;
}
```

### Newsletter as RawPage

**Key insight from brainstorm**: Newsletters are "just pages" to the extraction library.

Each incoming email becomes:

```rust
RawPage {
    url: format!("newsletter:{}:{}", source_id, message_id),
    content: email_body_as_markdown,
    title: Some(email_subject),
    content_type: Some("text/markdown"),
    fetched_at: email_received_at,
    metadata: {
        "platform": "newsletter",
        "sender": email_sender,
        "subject": email_subject,
        "message_id": message_id,
        "received_at": email_received_at.to_rfc3339(),
    }
}
```

### Extraction Library Query Pattern

**Current pattern** (`/packages/extraction/src/ingestors/firecrawl.rs`):

1. Takes a `DiscoverConfig` with starting URL
2. Returns `Vec<RawPage>` — these get stored in the database
3. Downstream code queries `extraction_pages` by `site_url`

**For newsletters**, you'll need:

```rust
// Modified query in domains/crawling to handle newsletter: URLs
// Current: assumes https:// prefix
// New: also handle newsletter: prefix

SELECT * FROM page_summaries
WHERE site_url LIKE 'newsletter:%'
  AND site_url = 'newsletter:' || source_id::TEXT
```

**File to modify**: Check `/packages/server/src/domains/crawling/` for where `site_url` is queried — likely in the post extraction pipeline.

### Ingestor Trait Integration

You **do not need to implement Ingestor** for newsletters because:
- Emails come via webhook (push, not pull)
- Ingestor is for active discovery (websites, social feeds)
- Newsletters are passive ingestion → directly insert as RawPage via webhook handler

But for consistency, you could create a `NewsletterIngestor` that acts as a dummy/replay ingestor for testing/replaying old emails.

### Extraction Pipeline Stages (No Changes Needed)

The existing 3-pass post extraction pipeline applies as-is:

1. **Pass 1 - Narrative Extraction**: Extract posts from newsletter content (same LLM prompt)
2. **Pass 2 - Deduplication**: Detect duplicate posts across newsletters and websites
3. **Pass 3 - Agentic Investigation**: Researcher fills in missing details (same as websites)

**Reference**: `/docs/architecture/CURATOR_PIPELINE.md` → extraction is Phase 2 of the curator workflow.

---

## 3. Webhook Handling Patterns (No Existing Implementation)

### What Exists

- No webhook infrastructure yet in the codebase
- This is new capability

### Recommended Pattern (from similar systems)

**Architecture**:

```
Postmark Inbound Webhook
    ↓ HTTP POST /api/webhooks/postmark/inbound
Router (validate signature)
    ↓
Handler (extract email data)
    ↓
Create RawPage
    ↓
Store in page_summaries
    ↓
Trigger ExtractOrgPostsWorkflow
```

### Key Implementation Points

1. **Webhook signature validation** (Critical for security)
   - Postmark signs webhooks with HMAC-SHA256
   - Validate using shared secret before processing
   - Reject unsigned requests immediately

2. **Idempotency** (Critical for reliability)
   - Email `message_id` uniqueness key in `page_summaries`
   - Store content hash to detect duplicates within same subscription
   - Webhook can be called multiple times for same email

3. **Error handling**
   - Log failed email ingestions without blocking other subscriptions
   - Don't let one bad email prevent others from processing
   - Return 200 OK to Postmark even if processing fails (prevent retries of bad data)

4. **Rate limiting** (Implicit via job scheduling)
   - Extraction pipeline's existing rate limiting applies
   - No additional webhook rate limiting needed (Postmark handles that)

---

## 4. Restate Workflow Integration Pattern

### Existing Workflow Examples

**Files**:
- `/packages/server/src/domains/source/restate/workflows/ingest_source.rs` — unified ingestion workflow
- `/packages/server/src/domains/crawling/restate/workflows/crawl_website.rs` — website crawl workflow

### Workflow Pattern for Newsletter Subscription

```rust
#[restate_sdk::workflow]
pub trait SubscribeNewsletterWorkflow {
    async fn run(req: SubscribeNewsletterRequest) -> Result<SubscribeNewsletterResult, HandlerError>;
}

pub struct SubscribeNewsletterRequest {
    pub newsletter_source_id: Uuid,  // newly created newsletter_sources record
    pub form_url: String,             // the signup form detected on website
}
```

**Flow** (durable with Restate):

1. Generate unique ingest email address
2. Update `newsletter_sources.ingest_email`
3. Launch headless Chrome to submit form
4. Wait for confirmation email (polling Postmark webhook)
5. Transition status: `subscribing` → `pending_confirmation` → `active`

### Key Pattern Notes

- **Wrap in ctx.run()**: All external calls (Chrome automation, DB updates) go in durable blocks
- **Activities return simple data**: Chrome activity returns `{ success, error }`
- **No events**: Workflows return result types directly, not domain events
- **Arc<ServerDeps> for deps**: Efficient cloning across workflow calls
- **impl_restate_serde!() on request/response**: Custom serialization

**Reference code**:

```rust
// From ingest_source.rs pattern
pub struct SubscribeNewsletterWorkflowImpl {
    deps: Arc<ServerDeps>,
}

impl SubscribeNewsletterWorkflowImpl {
    pub fn with_deps(deps: Arc<ServerDeps>) -> Self {
        Self { deps }
    }
}

impl SubscribeNewsletterWorkflow for SubscribeNewsletterWorkflowImpl {
    async fn run(
        &self,
        ctx: WorkflowContext<'_>,
        req: SubscribeNewsletterRequest,
    ) -> Result<SubscribeNewsletterResult, HandlerError> {
        // Load source (durable)
        let source = ctx
            .run(|| async {
                Source::find_by_id(&req.newsletter_source_id, &self.deps.db_pool)
                    .await
                    .map_err(Into::into)
            })
            .await?;

        // Submit form with headless Chrome (durable)
        let submit_result = ctx
            .run(|| async {
                activities::submit_newsletter_form(
                    &source.organization_id,
                    &req.form_url,
                    &self.deps,
                )
                .await
            })
            .await?;

        Ok(SubscribeNewsletterResult { success: submit_result })
    }
}
```

---

## 5. Database Extraction Pages Pattern

### Current Tables

**page_summaries** (from migration `000025_create_intelligent_crawler_tables.sql`):

```sql
CREATE TABLE page_summaries (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    site_url TEXT NOT NULL,  -- "https://example.com" or "newsletter:{source_id}"
    url TEXT NOT NULL UNIQUE,  -- full page identifier
    title TEXT,
    summary TEXT,
    content TEXT,
    fetched_via TEXT,  -- 'http', 'firecrawl', 'apify', 'postmark', etc.
    created_at TIMESTAMPTZ DEFAULT NOW()
);
```

### Newsletter Entry Pattern

```sql
INSERT INTO page_summaries (site_url, url, title, summary, content, fetched_via, created_at)
VALUES (
    'newsletter:' || source_id::TEXT,
    'newsletter:' || source_id::TEXT || ':' || message_id,
    email_subject,
    NULL,  -- summary filled in during extraction pass
    email_body_markdown,
    'postmark',
    NOW()
);
```

### Gotcha: site_url Queries

The extraction pipeline queries pages by `site_url` (grouping identifier). Your code must handle:

```sql
-- Website query (existing)
SELECT * FROM page_summaries WHERE site_url = 'https://example.com'

-- Newsletter query (new)
SELECT * FROM page_summaries WHERE site_url LIKE 'newsletter:%'
  AND site_url = 'newsletter:' || source_id::TEXT
```

**Files to audit**:
- Check all uses of `site_url` in `/packages/server/src/domains/crawling/`
- Ensure they handle both `https://` and `newsletter:` prefixes

---

## 6. Admin UI Pattern (GraphQL Layer)

### Current Architecture (from GraphQL migration plan)

**Path**: `/packages/shared/graphql/` (Next.js GraphQL BFF)

```
admin-app (Next.js)
    ↓ urql + GraphQL
    /api/graphql (GraphQL Yoga server in Next.js)
    ↓ RestateClient
    Restate Runtime
    ↓ HTTP
Rust Workflows
```

### Source Queries/Mutations (Model)

**From the migration plan**, `Sources` domain needs:

**Queries**:
```graphql
type Query {
    sources(filter: SourceFilter): [Source!]!
    source(id: ID!): Source
    sourceSnapshots(sourceId: ID!): [Snapshot!]!
}

type Source {
    id: ID!
    sourceType: String!  # 'website', 'instagram', 'facebook', 'tiktok', 'newsletter'
    url: String
    organizationId: ID
    organization: Organization
    status: String  # pending_review, approved, rejected, suspended
    active: Boolean!

    # Type-specific (union resolution)
    ... on WebsiteSource { domain, maxCrawlDepth, ... }
    ... on SocialSource { platform, handle, ... }
    ... on NewsletterSource { ingestEmail, subscriptionStatus, confirmationUrl, ... }
}
```

**Mutations**:
```graphql
type Mutation {
    # Generic source ops
    approveSource(id: ID!): Source!
    rejectSource(id: ID!, reason: String!): Source!
    deactivateSource(id: ID!): Source!

    # Newsletter-specific
    createNewsletterSource(organizationId: ID!, detectedUrl: String!): NewsletterSource!
    subscribeNewsletter(sourceId: ID!, formUrl: String!): WorkflowStatus!
    confirmNewsletterSubscription(sourceId: ID!): NewsletterSource!
    deactivateNewsletterSubscription(sourceId: ID!): NewsletterSource!
}
```

### Implementation Notes

1. **GraphQL types are inline SDL** in `schema.ts` (not .graphql files yet, per migration plan)
2. **Resolver per domain**: Will need `resolvers/source.ts` (or extend existing)
3. **Newsletter is a union/interface**: `SourceUnion = WebsiteSource | SocialSource | NewsletterSource`
4. **Long-running ops return workflowId**: Subscribe returns `WorkflowStatus { workflowId, status }`, client polls via `workflowStatus(id: ID!)`
5. **Auth: requireAdmin on all mutations** — per security audit findings

### Admin UI Components (Next.js)

**Pages to create/modify**:

1. `/admin/sources` — flat list of all sources (unified view)
   - Filter by type, org, status
   - Show newsletter status (subscription_status column)

2. `/admin/sources/[id]` — source detail page
   - For newsletter_sources: show ingest_email, subscription_status, confirmationUrl
   - "Confirm" button for pending_confirmation subscriptions
   - "Deactivate" button to stop ingestion

3. `/admin/organizations/[id]/sources` — org's newsletter sources
   - "Subscribe to Newsletter" button (triggers SubscribeNewsletterWorkflow)
   - Lists detected newsletters with signup URLs

---

## 7. Browser Automation (Headless Chrome) Pattern

### No Existing Implementation

- Codebase has no headless Chrome integration yet
- **Recommended**: Use Playwright or Puppeteer (both battle-tested)

### Recommended Activity Pattern

```rust
// domains/newsletters/activities/submit_form.rs

pub async fn submit_newsletter_form(
    org_id: Uuid,
    form_url: &str,
    ingest_email: &str,  // {uuid}@ingest.mntogether.org
    deps: &ServerDeps,
) -> Result<FormSubmitResult> {
    // Use Playwright or Puppeteer CLI
    // Navigate to form_url
    // Find email input (common patterns: id="email", name="email", etc.)
    // Fill with ingest_email
    // Submit form
    // Return { success, redirected_url, error_if_any }
}
```

### Gotchas

1. **Timeouts**: Forms can be slow; use generous timeouts (15-30s)
2. **JavaScript rendering**: Simple `curl` won't work; need headless browser
3. **Captchas**: Will fail on captcha-protected forms (acceptable — log and skip)
4. **Honeypot fields**: Some forms have hidden fields designed to catch bots; Playwright handles this naturally
5. **Form detection**: Assumes form is on the detected_signup_url; some orgs may not have public signup

---

## 8. Postmark Integration Pattern

### No Existing Postmark Integration

- New capability
- Postmark handles SMTP inbound and webhooks

### Setup Steps (Infrastructure)

1. **Configure inbound domain** on Postmark dashboard
   - Inbound domain: `ingest.mntogether.org` (separate from main domain)
   - Catch-all rule: send all email to webhook endpoint
   - Webhook URL: `https://mntogether.org/api/webhooks/postmark/inbound`
   - Webhook signing enabled (use shared secret for HMAC validation)

2. **DNS configuration**
   - MX record pointing to Postmark
   - SPF record for domain reputation

### Webhook Payload Format

Postmark sends JSON webhook with email metadata:

```json
{
  "MessageStream": "inbound",
  "From": "newsletter@orgsite.org",
  "FromFull": { "Email": "...", "Name": "..." },
  "To": "uuid-here@ingest.mntogether.org",
  "ToFull": [{ "Email": "uuid-here@ingest.mntogether.org", "Name": "" }],
  "Cc": "...",
  "Bcc": "...",
  "Subject": "Newsletter: February 2026",
  "MessageID": "...",
  "ReplyTo": "...",
  "Date": "2026-02-13T10:00:00Z",
  "TextBody": "...",
  "HtmlBody": "<html>...</html>",
  "MailboxHash": "...",
  "Tag": "...",
  "Headers": [...],
  "Attachments": [...]
}
```

**Key fields for newsletter ingestion**:
- `MessageID` — unique identifier (use as dedup key)
- `To` — route to subscription by parsing UUID
- `Subject`, `TextBody`, `HtmlBody` — email content
- `Date` — received timestamp
- `From` — sender info for metadata

---

## 9. Security & Rate Limiting Patterns

### From Unified Sources Pattern

**No explicit rate limiting on sources** — relying on:
1. **Scheduled scraping** (`scrape_frequency_hours`) — admin controls per-source frequency
2. **Crawl rate limits** on website_sources (`crawl_rate_limit_seconds`)

For newsletters:
- **No crawl rate limiting needed** (emails arrive as they come)
- **Postmark rate limiting** is automatic (they handle spam, rate limits)
- **Extraction pipeline rate limiting** applies downstream (shared with websites/social)

### Webhook Security

1. **HMAC signature validation** (Postmark signs all webhooks)
2. **Nonce/timestamp check** (prevent replay attacks)
3. **IP whitelist** (optional, Postmark publishes IP ranges)

---

## 10. Testing Patterns & Gotchas

### Key Test Points

1. **Migration test**: Verify class table inheritance with FK constraints
2. **Webhook idempotency**: Send same email twice, ensure it's not stored twice
3. **Form submission**: Mock Playwright, test form field detection
4. **Extraction pipeline**: Verify newsletter RawPage works through 3-pass extraction
5. **Admin UI**: GraphQL queries for newsletter sources, mutations for subscribe/confirm

### Gotchas from Existing Code

1. **Nullable fields in Rust models**: Use `Option<T>` for optional columns like `confirmation_url`, `last_email_received_at`
   - From SQLx rules: `#[derive(FromRow)]` handles this automatically
   - No need for `Option<Option<T>>` — SQLx is smart

2. **String representation in URLs**: `newsletter:{source_id}:{message_id}` is a made-up format
   - Ensure parsing is consistent everywhere
   - Consider validation/normalization in query builders

3. **Metadata JSON in page_summaries**: Keep metadata flat (string keys/values)
   - Easier to query and maintain than nested JSONB
   - Follow existing pattern from extraction library

4. **Workflow state in database**: Newsletter_sources.subscription_status is the source of truth
   - Not stored in Restate workflow state
   - Workflow writes back to DB on completion

---

## 11. Timeline Dependency Graph

**Suggested implementation order** (based on pattern dependencies):

```
1. Database (migrations)
   └─→ 2. Model + Queries (data access layer)
       └─→ 3. Webhook handler (receive emails)
       └─→ 4. Extraction integration (treat as RawPage)
           └─→ 5. Restate workflow (form submission + confirm)
               └─→ 6. Admin GraphQL + UI (manage subscriptions)
```

**Rationale**:
- Database must exist before models can query it
- Models before workflows (workflows call model methods)
- Webhook can work in parallel with workflow (independent)
- Extraction integration minimal changes (just query updates)
- Admin UI is last (depends on everything working)

---

## 12. Files to Create/Modify

### New Files (Expected)

**Database**:
- `packages/server/migrations/000XXX_add_newsletter_sources.sql`

**Domain Model**:
- `packages/server/src/domains/newsletters/` (new domain directory)
- `packages/server/src/domains/newsletters/models/newsletter_source.rs`
- `packages/server/src/domains/newsletters/activities/submit_form.rs`
- `packages/server/src/domains/newsletters/restate/workflows/subscribe_newsletter.rs`

**Webhook Handler**:
- `packages/server/src/api/webhooks/postmark.rs`
- `packages/server/src/api/routes.rs` (register webhook endpoint)

**GraphQL**:
- `packages/shared/graphql/resolvers/source.ts` (extend existing, add newsletter-specific queries/mutations)
- `packages/shared/graphql/schema.ts` (add NewsletterSource type)

**Admin UI**:
- `packages/admin-app/app/admin/sources/page.tsx` (modify to include newsletters)
- `packages/admin-app/app/admin/sources/[id]/page.tsx` (newsletter detail view)

### Modified Files (Expected)

**Database queries**:
- `packages/server/src/domains/crawling/` (audit `site_url` queries for `newsletter:` prefix)

**Workflow registration**:
- `packages/server/src/bin/server.rs` (register SubscribeNewsletterWorkflow)

**Extraction pipeline**:
- Check if any post extraction prompts need newsletter-aware context tweaks

**Admin UI**:
- `packages/admin-app/lib/hooks/useSource` (if exists) — extend with newsletter support

---

## Summary: Key Patterns to Follow

1. ✅ **Class table inheritance**: Match `website_sources` + `social_sources` pattern
2. ✅ **RawPage abstraction**: Newsletters are just pages to the extraction library
3. ✅ **Restate workflows**: Durable execution for form submission + confirmation flow
4. ✅ **GraphQL BFF**: Expose newsletter operations via GraphQL mutations
5. ✅ **Webhook signature validation**: HMAC-SHA256 from Postmark
6. ✅ **Idempotency via message_id**: Dedup incoming emails
7. ✅ **No new extraction code**: Reuse existing 3-pass pipeline
8. ✅ **Admin UI unification**: Extend `/admin/sources` to include newsletters

---

## Open Questions from Brainstorm

From the newsletter brainstorm document, these remain open:

1. **Rate limiting**: Should we throttle how many newsletters we process per org per day?
2. **Retention**: How long do we keep raw newsletter emails in page_summaries?
3. **Unsubscribe**: Do we need automated unsubscribe capability, or is deactivating the ingest address sufficient?
4. **Multiple newsletters per org**: Some orgs have several newsletters — handle as separate sources or group?

**Recommend**: Decide these during planning phase, add to feature spec.

---

## References

- **Unified Sources Pattern**: `/docs/architecture/DATABASE_SCHEMA.md` (Section: Source System)
- **Migration 000149**: `/packages/server/migrations/000149_create_unified_sources.sql` (class table inheritance implementation)
- **Ingestor Trait**: `/packages/extraction/src/traits/ingestor.rs` (RawPage abstraction)
- **Curator Pipeline**: `/docs/architecture/CURATOR_PIPELINE.md` (3-pass post extraction)
- **Restate Pattern**: `/packages/server/src/domains/source/restate/workflows/ingest_source.rs`
- **GraphQL Migration Plan**: `/docs/plans/2026-02-13-refactor-admin-app-graphql-migration-plan.md`
- **Newsletter Brainstorm**: `/docs/brainstorms/2026-02-13-newsletter-ingestion-brainstorm.md`
- **Post Extraction Prompt**: `/docs/brainstorms/2026-02-11-focused-post-extraction-brainstorm.md` (extraction pipeline stages)

