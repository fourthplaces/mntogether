# Phase 4.4: Email Newsletter via Amazon SES

> **Pre-migration design doc.** Written when the plan routed all backend
> work through Restate. Restate was removed on 2026-03-17 (see
> `ARCHITECTURE_DECISIONS.md` Decision 4). References below to
> "Restate handlers / services / workflows" and `domains/*/restate/`
> directories correspond to Axum HTTP handlers in
> `src/api/routes/{domain}.rs` in the current codebase. If/when this
> feature is built, the "durable SendNewsletterWorkflow" section should
> be revisited — durability will likely need a job queue rather than
> an in-process handler.

**Status:** Deferred (see [ARCHITECTURE_DECISIONS.md](ARCHITECTURE_DECISIONS.md), Decision 3)
**Priority:** 4 of 4 (most infrastructure, least dependency on other subprojects)
**Depends on:** Phase 3 (Edition System — complete), editions must be publishable

> **Deferred — not superseded.** The product vision (weekly edition preview emails per county) remains valid. Full in-house email infrastructure (SES integration, subscribers table, send workflow, batch processing) is deferred. When the time comes, it uses Amazon SES — the same AWS account and Pulumi IaC stack the rest of the infrastructure runs on.

---

## Context

When an editor publishes a weekly edition, there is no mechanism to notify subscribers by email. The broadsheet exists in the admin UI and (eventually) the public web-app, but there is no email distribution channel.

This subproject adds outbound email: generating a condensed email version of a published edition and sending it to subscribers via Amazon SES.

---

## Architecture Decisions

### 1. Amazon SES v2 API (not a third-party email service)

The entire infrastructure runs on AWS via Pulumi. SES keeps email delivery in the same account — no external API keys, no third-party vendor relationship, no additional billing. SES v2's `SendBulkEmail` API handles per-recipient personalization (unsubscribe links) and supports up to 50 messages per call. For larger lists, the workflow batches automatically.

Pulumi provisions the SES configuration set, verified domain identity (mntogether.org), and DKIM records in the existing `core` stack. The ECS task role gets `ses:SendEmail` and `ses:SendBulkEmail` permissions — no secrets to rotate.

### 2. Simple `subscribers` table, not a mailing list platform

For MVP we need: email address, county preference (which edition to receive), active/unsubscribed status, and a confirm token for unsubscribe links. We do not need segments, A/B testing, or complex automation. If subscriber management grows complex later, migrate to a dedicated service.

### 3. Server-side HTML rendering in Rust

Email HTML requires inline CSS (no external stylesheets). Generating this in Rust keeps all content rendering server-side and avoids a separate template engine. A pure function maps edition data (rows, slots, post titles/summaries) into structured HTML. The `format!` macro and string builders are sufficient for a clean, single-column email layout — no heavyweight template engine needed.

### 4. Durable `SendNewsletterWorkflow` via Restate

Sending to potentially thousands of subscribers should be durable and resumable. A Restate workflow provides automatic retries, progress tracking, and idempotency per invocation. The workflow: render HTML → load subscribers → create send record → batch-send via SES → update progress.

### 5. `BaseSesService` trait for testability

Following the established pattern: `BaseTwilioService` trait in `kernel/traits.rs` with `TwilioAdapter` in `kernel/deps.rs`. Create `BaseSesService` trait and `SesAdapter` the same way. Tests mock the trait without hitting the real API.

---

## Infrastructure Changes (Pulumi)

### Add to `infra/packages/core/index.ts`

```typescript
// ─── SES Domain Identity ─────────────────────────────────
const sesIdentity = new aws.ses.DomainIdentity("ses-identity", {
    domain: "mntogether.org",
});

const sesDkim = new aws.ses.DomainDkim("ses-dkim", {
    domain: sesIdentity.domain,
});

// Add DKIM CNAME records to Route53
sesDkim.dkimTokens.apply(tokens => {
    tokens.forEach((token, i) => {
        new aws.route53.Record(`ses-dkim-${i}`, {
            zoneId: hostedZone.zoneId,
            name: `${token}._domainkey.mntogether.org`,
            type: "CNAME",
            records: [`${token}.dkim.amazonses.com`],
            ttl: 600,
        });
    });
});

// Configuration set for tracking
const sesConfigSet = new aws.sesv2.ConfigurationSet("newsletter-config-set", {
    configurationSetName: "newsletter",
    sendingOptions: { sendingEnabled: true },
});

export const sesIdentityArn = sesIdentity.arn;
export const sesConfigSetName = sesConfigSet.configurationSetName;
```

### Add to `infra/packages/server/index.ts` (ECS task role)

```typescript
// SES send permissions for newsletter
new aws.iam.RolePolicyAttachment("ses-send-policy", {
    role: taskRole,
    policyArn: new aws.iam.Policy("ses-send", {
        policy: JSON.stringify({
            Version: "2012-10-17",
            Statement: [{
                Effect: "Allow",
                Action: ["ses:SendEmail", "ses:SendBulkEmail"],
                Resource: "*",
                Condition: {
                    StringEquals: {
                        "ses:FromAddress": "newsletter@mntogether.org",
                    },
                },
            }],
        }),
    }).arn,
});
```

---

## Database Changes

### Migration: `packages/server/migrations/000178_create_newsletter_system.sql`

```sql
-- ─── Subscribers ─────────────────────────────────────────

CREATE TABLE subscribers (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email           TEXT NOT NULL,
    county_id       UUID REFERENCES counties(id),
    status          TEXT NOT NULL DEFAULT 'active'
                    CHECK (status IN ('active', 'unsubscribed', 'bounced')),
    confirm_token   TEXT UNIQUE DEFAULT encode(gen_random_bytes(32), 'hex'),
    confirmed_at    TIMESTAMPTZ,
    unsubscribed_at TIMESTAMPTZ,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- One subscription per email per county (NULL county = all counties)
CREATE UNIQUE INDEX idx_subscribers_email_county
    ON subscribers (email, COALESCE(county_id, '00000000-0000-0000-0000-000000000000'));

CREATE INDEX idx_subscribers_county_active
    ON subscribers (county_id) WHERE status = 'active';

-- ─── Newsletter Sends ────────────────────────────────────

CREATE TABLE newsletter_sends (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    edition_id      UUID NOT NULL REFERENCES editions(id),
    subject         TEXT NOT NULL,
    html_body       TEXT NOT NULL,
    recipient_count INT NOT NULL DEFAULT 0,
    sent_count      INT NOT NULL DEFAULT 0,
    failed_count    INT NOT NULL DEFAULT 0,
    status          TEXT NOT NULL DEFAULT 'pending'
                    CHECK (status IN ('pending', 'sending', 'completed', 'failed')),
    error_message   TEXT,
    started_at      TIMESTAMPTZ,
    completed_at    TIMESTAMPTZ,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_newsletter_sends_edition ON newsletter_sends(edition_id);
```

---

## Backend Changes

### New domain: `packages/server/src/domains/newsletter/`

```
newsletter/
├── mod.rs
├── models/
│   ├── mod.rs
│   ├── subscriber.rs
│   └── newsletter_send.rs
├── activities/
│   ├── mod.rs
│   ├── render.rs
│   └── send.rs
└── restate/
    ├── mod.rs
    ├── services/
    │   ├── mod.rs
    │   └── newsletter.rs
    └── workflows/
        ├── mod.rs
        └── send_newsletter.rs
```

### Model: `subscriber.rs`

```rust
#[derive(Debug, Clone, sqlx::FromRow, Serialize, Deserialize)]
pub struct Subscriber {
    pub id: Uuid,
    pub email: String,
    pub county_id: Option<Uuid>,
    pub status: String,
    pub confirm_token: Option<String>,
    pub confirmed_at: Option<DateTime<Utc>>,
    pub unsubscribed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Subscriber {
    pub async fn create(email: &str, county_id: Option<Uuid>, pool: &PgPool) -> Result<Self>;
    pub async fn find_active_by_county(county_id: Uuid, pool: &PgPool) -> Result<Vec<Self>>;
    pub async fn find_all_active(pool: &PgPool) -> Result<Vec<Self>>;
    pub async fn list(county_id: Option<Uuid>, status: Option<&str>, limit: i64, offset: i64, pool: &PgPool) -> Result<(Vec<Self>, i64)>;
    pub async fn unsubscribe(id: Uuid, pool: &PgPool) -> Result<Self>;
    pub async fn unsubscribe_by_token(token: &str, pool: &PgPool) -> Result<Self>;
    pub async fn count_active(county_id: Option<Uuid>, pool: &PgPool) -> Result<i64>;
    pub async fn delete(id: Uuid, pool: &PgPool) -> Result<()>;
}
```

### Model: `newsletter_send.rs`

```rust
#[derive(Debug, Clone, sqlx::FromRow, Serialize, Deserialize)]
pub struct NewsletterSend {
    pub id: Uuid,
    pub edition_id: Uuid,
    pub subject: String,
    pub html_body: String,
    pub recipient_count: i32,
    pub sent_count: i32,
    pub failed_count: i32,
    pub status: String,
    pub error_message: Option<String>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

impl NewsletterSend {
    pub async fn create(edition_id: Uuid, subject: &str, html_body: &str, recipient_count: i32, pool: &PgPool) -> Result<Self>;
    pub async fn update_progress(id: Uuid, sent_count: i32, failed_count: i32, status: &str, pool: &PgPool) -> Result<Self>;
    pub async fn find_by_edition(edition_id: Uuid, pool: &PgPool) -> Result<Vec<Self>>;
    pub async fn find_latest_by_edition(edition_id: Uuid, pool: &PgPool) -> Result<Option<Self>>;
}
```

### Trait: `packages/server/src/kernel/traits.rs`

```rust
#[async_trait]
pub trait BaseSesService: Send + Sync {
    /// Send a batch of emails via SES v2.
    async fn send_batch(&self, messages: Vec<SesMessage>) -> Result<Vec<SesSendResult>>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SesMessage {
    pub to: String,
    pub subject: String,
    pub html_body: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SesSendResult {
    pub to: String,
    pub message_id: Option<String>,
    pub success: bool,
    pub error: Option<String>,
}
```

### Adapter: `packages/server/src/kernel/deps.rs`

Following the `TwilioAdapter` pattern:

```rust
use aws_sdk_sesv2 as sesv2;

pub struct SesAdapter {
    client: sesv2::Client,
    from_email: String,
    config_set: String,
}

impl SesAdapter {
    pub async fn new(from_email: String, config_set: String) -> Self {
        let config = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;
        Self {
            client: sesv2::Client::new(&config),
            from_email,
            config_set,
        }
    }
}

#[async_trait]
impl BaseSesService for SesAdapter {
    async fn send_batch(&self, messages: Vec<SesMessage>) -> Result<Vec<SesSendResult>> {
        let mut results = Vec::with_capacity(messages.len());

        for msg in &messages {
            let result = self.client
                .send_email()
                .from_email_address(&self.from_email)
                .destination(
                    sesv2::types::Destination::builder()
                        .to_addresses(&msg.to)
                        .build()
                )
                .content(
                    sesv2::types::EmailContent::builder()
                        .simple(
                            sesv2::types::Message::builder()
                                .subject(
                                    sesv2::types::Content::builder()
                                        .data(&msg.subject)
                                        .charset("UTF-8")
                                        .build()
                                        .expect("subject content")
                                )
                                .body(
                                    sesv2::types::Body::builder()
                                        .html(
                                            sesv2::types::Content::builder()
                                                .data(&msg.html_body)
                                                .charset("UTF-8")
                                                .build()
                                                .expect("html content")
                                        )
                                        .build()
                                )
                                .build()
                        )
                        .build()
                )
                .configuration_set_name(&self.config_set)
                .send()
                .await;

            match result {
                Ok(output) => results.push(SesSendResult {
                    to: msg.to.clone(),
                    message_id: output.message_id,
                    success: true,
                    error: None,
                }),
                Err(e) => results.push(SesSendResult {
                    to: msg.to.clone(),
                    message_id: None,
                    success: false,
                    error: Some(e.to_string()),
                }),
            }
        }

        Ok(results)
    }
}
```

### ServerDeps: `packages/server/src/kernel/deps.rs`

Add to `ServerDeps` struct:
```rust
pub ses: Option<Arc<dyn BaseSesService>>,
```

Make it `Option` — when SES is not configured (no AWS credentials or feature disabled), newsletter features are disabled.

### Activity: `render.rs`

```rust
/// Render a published edition as an HTML email body.
pub fn render_newsletter_html(
    edition: &Edition,
    county: &County,
    rows: &[EditionRowWithSlots],
    unsubscribe_url_template: &str,  // "https://mntogether.org/unsubscribe?token={{token}}"
) -> (String, String) {  // (subject, html_body)
    let subject = format!("{} — Week of {}", county.name, edition.period_start);
    let html = build_email_html(edition, county, rows, unsubscribe_url_template);
    (subject, html)
}
```

The HTML renderer produces a single-column email layout with:
- Header: county name, edition date range
- Per-row section: row template name as section heading
- Per-slot: post title (bold), summary or truncated description
- Footer: unsubscribe link (tokenized), MN Together branding

All CSS is inline. Use tables for email client compatibility.

### Activity: `send.rs`

```rust
/// Send newsletter to all active subscribers for a county.
pub async fn send_newsletter_batch(
    send_id: Uuid,
    subscribers: &[Subscriber],
    subject: &str,
    html_template: &str,  // Contains {{unsubscribe_url}} placeholder
    base_unsubscribe_url: &str,
    deps: &ServerDeps,
) -> Result<(i32, i32)> {  // (sent_count, failed_count)
    let ses = deps.ses.as_ref()
        .ok_or_else(|| anyhow::anyhow!("SES not configured"))?;

    let mut total_sent = 0;
    let mut total_failed = 0;

    // SES rate: 14 emails/sec by default; batch in chunks of 50
    for chunk in subscribers.chunks(50) {
        let messages: Vec<SesMessage> = chunk.iter().map(|sub| {
            let html = html_template.replace(
                "{{unsubscribe_url}}",
                &format!("{}?token={}", base_unsubscribe_url, sub.confirm_token.as_deref().unwrap_or("")),
            );
            SesMessage {
                to: sub.email.clone(),
                subject: subject.into(),
                html_body: html,
            }
        }).collect();

        let results = ses.send_batch(messages).await?;
        for r in &results {
            if r.success { total_sent += 1; } else { total_failed += 1; }
        }
    }

    Ok((total_sent, total_failed))
}
```

### Restate Service: `newsletter.rs`

```rust
#[restate_sdk::service]
pub trait NewsletterService {
    async fn list_subscribers(req: ListSubscribersRequest) -> Result<SubscriberListResult, HandlerError>;
    async fn add_subscriber(req: AddSubscriberRequest) -> Result<SubscriberResult, HandlerError>;
    async fn remove_subscriber(req: RemoveSubscriberRequest) -> Result<bool, HandlerError>;
    async fn preview_newsletter(req: PreviewNewsletterRequest) -> Result<NewsletterPreviewResult, HandlerError>;
    async fn list_sends(req: ListSendsRequest) -> Result<Vec<NewsletterSendResult>, HandlerError>;
    async fn trigger_send(req: TriggerSendRequest) -> Result<NewsletterSendResult, HandlerError>;
}
```

The `trigger_send` handler starts the `SendNewsletterWorkflow` asynchronously and returns the created `NewsletterSend` record.

### Restate Workflow: `send_newsletter.rs`

```rust
#[restate_sdk::workflow]
pub trait SendNewsletterWorkflow {
    async fn run(req: SendNewsletterRequest) -> Result<SendNewsletterResult, HandlerError>;
}

// Steps (each wrapped in ctx.run for durability):
// 1. Load edition with rows/slots
// 2. Load county
// 3. Render HTML
// 4. Load active subscribers for county
// 5. Create NewsletterSend record
// 6. Send in 50-recipient batches (SES rate limits)
// 7. Update progress after each batch
// 8. Mark send as completed
```

### Server registration: `packages/server/src/bin/server.rs`

```rust
// SES client — uses IAM task role credentials automatically
let ses: Option<Arc<dyn BaseSesService>> = if std::env::var("NEWSLETTER_ENABLED").unwrap_or_default() == "true" {
    let from = std::env::var("SES_FROM_EMAIL")
        .unwrap_or_else(|_| "newsletter@mntogether.org".into());
    let config_set = std::env::var("SES_CONFIG_SET")
        .unwrap_or_else(|_| "newsletter".into());
    Some(Arc::new(SesAdapter::new(from, config_set).await) as Arc<dyn BaseSesService>)
} else {
    None
};

// Add to ServerDeps::new(...)

// Register services
.bind(NewsletterServiceImpl::with_deps(deps.clone()).serve())
.bind(SendNewsletterWorkflowImpl::with_deps(deps.clone()).serve())
```

### Cargo dependency: `packages/server/Cargo.toml`

```toml
aws-sdk-sesv2 = "1"
aws-config = "1"
```

---

## GraphQL Changes

### Schema: `packages/shared/graphql/schema.ts`

Add types:

```graphql
type Subscriber {
  id: ID!
  email: String!
  countyId: ID
  countyName: String
  status: String!
  confirmedAt: String
  createdAt: String!
}

type SubscriberConnection {
  subscribers: [Subscriber!]!
  totalCount: Int!
}

type NewsletterSend {
  id: ID!
  editionId: ID!
  subject: String!
  recipientCount: Int!
  sentCount: Int!
  failedCount: Int!
  status: String!
  errorMessage: String
  startedAt: String
  completedAt: String
  createdAt: String!
}

type NewsletterPreview {
  subject: String!
  htmlBody: String!
  recipientCount: Int!
}
```

Add to `Query`:
```graphql
subscribers(countyId: ID, status: String, limit: Int, offset: Int): SubscriberConnection!
newsletterSends(editionId: ID, limit: Int): [NewsletterSend!]!
newsletterPreview(editionId: ID!): NewsletterPreview!
```

Add to `Mutation`:
```graphql
addSubscriber(email: String!, countyId: ID): Subscriber!
removeSubscriber(id: ID!): Boolean!
sendNewsletter(editionId: ID!): NewsletterSend!
```

### Resolver: `packages/shared/graphql/resolvers/newsletter.ts` (new file)

```typescript
export const newsletterResolvers = {
  Query: {
    subscribers: async (_parent, args, ctx) => {
      return ctx.restate.callService("Newsletter", "list_subscribers", {
        county_id: args.countyId,
        status: args.status,
        limit: args.limit,
        offset: args.offset,
      });
    },
    newsletterSends: async (_parent, args, ctx) => {
      return ctx.restate.callService("Newsletter", "list_sends", {
        edition_id: args.editionId,
        limit: args.limit,
      });
    },
    newsletterPreview: async (_parent, args, ctx) => {
      return ctx.restate.callService("Newsletter", "preview_newsletter", {
        edition_id: args.editionId,
      });
    },
  },
  Mutation: {
    addSubscriber: async (_parent, args, ctx) => {
      return ctx.restate.callService("Newsletter", "add_subscriber", {
        email: args.email,
        county_id: args.countyId,
      });
    },
    removeSubscriber: async (_parent, args, ctx) => {
      return ctx.restate.callService("Newsletter", "remove_subscriber", {
        id: args.id,
      });
    },
    sendNewsletter: async (_parent, args, ctx) => {
      return ctx.restate.callService("Newsletter", "trigger_send", {
        edition_id: args.editionId,
      });
    },
  },
};
```

Register in `resolvers/index.ts`:
```typescript
import { newsletterResolvers } from "./newsletter";
// Add to mergeResolvers array
```

---

## Frontend Changes

### New page: `packages/admin-app/app/admin/(app)/newsletter/page.tsx`

Two-section layout:

**Section 1: Subscribers**
- Table: email, county, status, subscribed date
- "Add Subscriber" form (email + county dropdown)
- Remove button per row
- Filter by county
- Count display

**Section 2: Send History**
- Table: edition period, subject, status, sent/failed counts, date
- Status badges: pending (yellow), sending (blue), completed (green), failed (red)

### New page: `packages/admin-app/app/admin/(app)/newsletter/preview/[editionId]/page.tsx`

- Fetches `newsletterPreview(editionId)` to get rendered HTML
- Shows subject line in a text field
- Renders HTML in a sandboxed iframe (`srcdoc` attribute)
- Displays recipient count
- "Send Newsletter" button → confirms, calls `sendNewsletter` mutation
- Back link to edition or newsletter page

### Modified page: `packages/admin-app/app/admin/(app)/editions/[id]/page.tsx`

For published editions, add a "Newsletter" section:
- If no send exists: "Send Newsletter" button → navigates to preview page
- If send exists: Shows last send status (sent X of Y, failed Z)
- Link to newsletter preview

### New queries: `packages/admin-app/lib/graphql/newsletter.ts`

```typescript
export const SubscribersQuery = graphql(`...`);
export const NewsletterSendsQuery = graphql(`...`);
export const NewsletterPreviewQuery = graphql(`...`);
export const AddSubscriberMutation = graphql(`...`);
export const RemoveSubscriberMutation = graphql(`...`);
export const SendNewsletterMutation = graphql(`...`);
```

### Sidebar: `packages/admin-app/components/admin/AdminSidebar.tsx`

Add "Newsletter" to the "Content" nav group, after "Editions".

### Config: `.env.example`

Add:
```
NEWSLETTER_ENABLED=false
SES_FROM_EMAIL=newsletter@mntogether.org
SES_CONFIG_SET=newsletter
```

No API keys needed — SES authenticates via IAM task role in ECS, and via default AWS credentials locally.

---

## Existing Code to Reuse

| What | Where | How |
|------|-------|-----|
| `BaseTwilioService` trait pattern | `kernel/traits.rs:30` | Template for `BaseSesService` |
| `TwilioAdapter` pattern | `kernel/deps.rs:24` | Template for `SesAdapter` |
| `ServerDeps` struct | `kernel/deps.rs:57` | Add `ses` field |
| `reqwest` HTTP client | Already in `Cargo.toml` | Not needed — SES uses AWS SDK |
| `Edition::find_by_id` | `edition.rs:55` | Load edition for rendering |
| `County::find_by_id` | `county.rs` | Load county for rendering |
| `EditionRow::find_by_edition` | `edition_row.rs` | Load rows for rendering |
| `EditionSlot::find_by_rows` | `edition_slot.rs` | Load slots for rendering |
| `Post::find_by_ids` | `post.rs:497` | Load post content for rendering |
| Restate workflow pattern | Existing workflows in codebase | `ctx.run()` for durability |
| `callService` resolver pattern | `resolvers/edition.ts` | Template for newsletter resolvers |
| `Badge`, `Button`, `Card` | `admin-app/components/ui/` | UI components |
| `CountiesQuery` | `admin-app/lib/graphql/editions.ts` | County dropdown for subscribers |

---

## Implementation Steps

1. **Pulumi**: Add SES domain identity, DKIM records, configuration set to `infra/packages/core/`
2. **Pulumi**: Add `ses:SendEmail` IAM policy to ECS task role in `infra/packages/server/`
3. **Migration**: Create `000178_create_newsletter_system.sql` (subscribers + newsletter_sends tables)
4. **Migration**: Run `make migrate`
5. **Cargo**: Add `aws-sdk-sesv2` and `aws-config` dependencies
6. **Model**: Create `Subscriber` with all CRUD methods
7. **Model**: Create `NewsletterSend` with CRUD and progress update
8. **Trait**: Add `BaseSesService` to `kernel/traits.rs`
9. **Adapter**: Add `SesAdapter` to `kernel/deps.rs`
10. **ServerDeps**: Add `ses: Option<Arc<dyn BaseSesService>>`
11. **Activity**: Create `render.rs` — HTML email renderer
12. **Activity**: Create `send.rs` — batch send logic
13. **Restate**: Create `NewsletterService` with subscriber CRUD + preview + trigger
14. **Restate**: Create `SendNewsletterWorkflow` for durable send
15. **Server**: Register new service and workflow in `server.rs`
16. **Server**: Initialize SES client from env/IAM
17. **GraphQL**: Add types, queries, mutations to `schema.ts`
18. **GraphQL**: Create `resolvers/newsletter.ts`, register in `index.ts`
19. **Frontend**: Create `lib/graphql/newsletter.ts` queries
20. **Frontend**: Create newsletter admin page
21. **Frontend**: Create newsletter preview page
22. **Frontend**: Update edition detail with newsletter section
23. **Frontend**: Update sidebar
24. **Config**: Update `.env.example`
25. **Codegen**: Run `yarn codegen` in admin-app
26. **Rebuild**: `docker compose up -d --build server`

---

## Verification

1. Run `pulumi up` for core stack — verify SES identity and DKIM records created
2. Run migration — verify `subscribers` and `newsletter_sends` tables exist
3. Add a subscriber via admin page — verify it appears in the list
4. Remove a subscriber — verify status changes to `unsubscribed`
5. Navigate to newsletter preview for a published edition — verify HTML renders in iframe
6. Subject line should read "{County Name} — Week of {date}"
7. Email HTML should list post titles and summaries from the edition
8. Send newsletter (SES sandbox mode) — verify send record created with correct counts
9. Verify send status updates from `pending` → `sending` → `completed`
10. Edition detail page shows newsletter send status for published editions
11. With `NEWSLETTER_ENABLED=false`, server starts without error (feature disabled)
12. In production, verify SES sends from `newsletter@mntogether.org` with DKIM passing
