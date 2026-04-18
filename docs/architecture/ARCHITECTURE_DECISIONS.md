# Architecture Decisions

**Date:** 2026-02-27
**Context:** Root Editorial is a fork of Root Signal, inheriting infrastructure designed for a different product. These decisions slim the architecture for what Root Editorial actually is: a CMS for community editorial teams publishing 87 county newspapers in Minnesota.

---

## Decision 1: Static Public Site

`packages/web-app` becomes a static site build deployed to CDN.

- **Build tool:** Next.js static export, or a lighter alternative (Astro, 11ty)
- **Hosting:** CloudFront + S3 (already provisioned in `infra/packages/web-app/`) — no Node.js runtime
- **Rebuild trigger:** Edition publish event triggers a rebuild and S3 sync + CloudFront invalidation
- **Consequence:** The public newspaper has zero server-side attack surface

---

## Decision 2: Lightweight Form Handling for Community Submissions

Public-facing forms (community submissions, tips, event listings) use a minimal serverless function rather than exposing the full backend.

- **Implementation:** API Gateway + Lambda function behind CloudFront, provisioned via Pulumi in the existing `web-app` stack
- **Lambda provides:** Input validation, rate limiting (API Gateway throttling), and a write to the Rust server's webhook endpoint
- **Data flow:** Form submission → API Gateway → Lambda → Rust server webhook → creates post with `submission_type='community', status='pending_approval'`
- **Consequence:** No custom form backend on the static site, no CAPTCHA infrastructure, minimal public API surface

---

## Decision 3: Minimal PII, Amazon SES for Email Delivery

Root Editorial minimizes PII storage — no user accounts, no browsing data, no profiles.

Email newsletters are a genuine product need: subscribers register an email to a county edition and receive a weekly preview of the published newspaper. But building the full email infrastructure (SES integration, subscribers table, send workflow, batch processing) is **deferred** — not needed for MVP.

When implemented, email delivery uses **Amazon SES** — the same AWS account and Pulumi IaC stack the rest of the infrastructure runs on. No external vendor relationship, no additional API keys to manage.

- **Approach:** A slim `subscribers` table (email + county_id + status) with delivery via SES v2. The ECS task role gets `ses:SendEmail` permissions automatically — no secrets to rotate. The email list is the only PII, and it's kept minimal.
- **Infrastructure:** SES domain identity, DKIM records, and configuration set are provisioned via Pulumi in the `core` stack.

RSS feeds per county serve as a zero-PII complement — readers who prefer feed readers get the same edition content without registering.

> **See also:** [`EMAIL_NEWSLETTER.md`](EMAIL_NEWSLETTER.md) — the detailed implementation plan. Its status is **Deferred**, not superseded. The product vision (weekly edition preview emails per county) remains valid; the implementation timing is TBD.

---

## Decision 4: All Backend Operations Are Plain Axum HTTP Handlers

**Superseded the original Restate decision (2026-03-17).** Every backend
operation — CRUD, queries, long-running work — is a plain Axum HTTP
handler. The architecture is
`Next.js → GraphQL resolvers → Axum HTTP (port 9080) → PostgreSQL`.
No workflow runtime, no service mesh.

- **CRUD features** (media library, post create/update, dashboard
  queries) are thin `async fn` handlers in `packages/server/src/api/routes/`
  that delegate to activity functions.
- **Long-running work** (newsletter send, edition generation, Signal
  integration): for dev today, synchronous calls inside the same
  handler; for production, revisit with a job queue when we have
  concrete durability needs. Don't pay for Restate-shaped complexity
  before there's a workload that demands it.
- **Keyed operations** (per-post writes): use row-level locks / `SELECT
  ... FOR UPDATE` when concurrency is real; otherwise just write.
- **Pattern:** GraphQL resolvers call `ctx.server.callService(...)` in
  `packages/shared/graphql/server-client.ts`, which issues a POST to
  the Rust server. Activities are pure functions taking `&ServerDeps`.
  SQL lives in models.

The original decision pushed everything through Restate for "one
mental model, one call pattern." In practice the durability benefits
weren't being used — our workloads are short request/response. The
SDK overhead, dual-serialization (`impl_restate_serde!` macro), and
extra runtime process weren't paying for themselves. Removed in the
pivot (see `ROOT_EDITORIAL_PIVOT.md`).

---

## Decision 5: Root Signal Integration via Webhook

Root Signal pushes content into Root Editorial via a simple webhook
endpoint — not a shared deployment or service mesh.

- **Endpoint:** One thin receiver validates a shared secret and inserts a post
- **Data:** Posts arrive as `submission_type='signal', status='pending_approval'`
- **Schema compatibility:** Both systems share the post schema where it matters, enabling clean data transfer
- **Consequence:** No complex service mesh, no shared infrastructure between the two systems

---

## Decision 6: Admin App Is the Only Dynamic Piece

- `packages/admin-app` stays as Next.js with server-side rendering, behind authentication
- `packages/web-app` is static CDN (Decision 1)
- **Attack surface:** Just the admin app + Rust backend, both behind auth
- **Monitoring:** Only one server to monitor, scale, and secure

---

## Decision 7: Two Apps Stay Separate

Admin and web have fundamentally different security concerns:

- **Web app:** Public, static, CDN-hosted, zero server. Anyone can read.
- **Admin app:** Authenticated, server-rendered, behind auth. Only editorial teams.

Separate packages are correct. But the web app's build system gets dramatically simpler — from a full Next.js server to a static export with no runtime.
