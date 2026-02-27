# Architecture Decisions

**Date:** 2026-02-27
**Context:** Root Editorial is a fork of Root Signal, inheriting infrastructure designed for a different product. These decisions slim the architecture for what Root Editorial actually is: a CMS for community editorial teams publishing 87 county newspapers in Minnesota.

---

## Decision 1: Static Public Site

`packages/web-app` becomes a static site build deployed to CDN.

- **Build tool:** Next.js static export, or a lighter alternative (Astro, 11ty)
- **Hosting:** Cloudflare Pages, Netlify, or Vercel — no Node.js runtime
- **Rebuild trigger:** Edition publish event triggers a rebuild
- **Consequence:** The public newspaper has zero server-side attack surface

---

## Decision 2: Platform Form Handling for Community Submissions

Public-facing forms (community submissions, tips, event listings) use the hosting platform's built-in form handling rather than a custom backend.

- **Examples:** Cloudflare Workers, Netlify Forms, Vercel Edge Functions
- **Platform provides:** Spam protection, rate limiting, bot detection
- **Data flow:** Form submission → platform → webhook → thin API endpoint → creates post with `submission_type='community', status='pending_approval'`
- **Consequence:** No custom form backend, no CAPTCHA infrastructure, no public API surface

---

## Decision 3: Minimal PII, External Email Delivery

Root Editorial minimizes PII storage — no user accounts, no browsing data, no profiles.

Email newsletters are a genuine product need: subscribers register an email to a county edition and receive a weekly preview of the published newspaper. But building full email infrastructure in-house (Postmark integration, subscribers table, send workflow, batch processing) is **deferred** — not needed for MVP.

When implemented, prefer an external email service that manages subscriber lists and delivery:

- **Option A — Fully external:** A service like Buttondown or Mailchimp holds the email list entirely. Root Editorial generates the email content (edition preview HTML) and pushes it to the service via API. Zero PII stored locally.
- **Option B — Minimal local storage:** A slim `subscribers` table (email + county_id + status) with external delivery via Postmark or SendGrid. The email list is the only PII, and it's kept minimal.

Either way, the email list is the only PII, and it's either externalized or kept minimal.

RSS feeds per county serve as a zero-PII complement — readers who prefer feed readers get the same edition content without registering.

> **See also:** [`phase4/EMAIL_NEWSLETTER.md`](phase4/EMAIL_NEWSLETTER.md) — the detailed implementation plan. Its status is **Deferred**, not superseded. The product vision (weekly edition preview emails per county) remains valid; the implementation timing and infrastructure approach are TBD.

---

## Decision 4: Bypass Restate for New CRUD Features

New features that are pure CRUD (media library, post create/update, dashboard queries) should talk to the database more directly rather than routing through Restate's durable execution framework.

- **Options:** GraphQL resolvers call Rust models via a simpler HTTP API, or Next.js API routes with direct DB access
- **Keep Restate for:** Genuine workflows — Signal integration, edition generation, any multi-step async operations that benefit from durability guarantees
- **Existing features:** Not refactored. Restate-routed features work fine and aren't worth rewriting.
- **Philosophy:** Gradual simplification, not a rewrite. Each new feature chooses the lightest path that fits.

---

## Decision 5: Root Signal Integration via Webhook

Root Signal pushes content into Root Editorial via a simple webhook endpoint — not a shared Restate deployment or service mesh.

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
