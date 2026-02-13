---
date: 2026-02-13
topic: newsletter-ingestion
---

# Newsletter Ingestion: Third Source Type for Organizations

## What We're Building

Newsletters as a third source type alongside websites and social media accounts. Organizations' newsletters get ingested via Postmark inbound email, stored as extraction pages, and fed through the existing post extraction pipeline — same code path as website scrapes.

## Why This Approach

- Many organizations communicate via newsletters (events, programs, opportunities) that aren't on their website
- The extraction library is already domain-agnostic — `extraction_pages` has no HTTP assumptions, `site_url` is just a grouping string, `content` is just text
- Reusing the post extraction pipeline means zero new AI/extraction code — newsletters are just "pages" to the extractor
- Postmark inbound email is a battle-tested, simple webhook-based approach

## Full Flow

### 1. Newsletter Detection (during org extraction)

- During the existing org extraction pass (already runs after website crawl), the LLM identifies newsletter signup URLs/forms from crawled pages
- Stored on the organization/source record as `detected_newsletter_url`
- No new infrastructure — just an addition to the org extraction prompt

### 2. Admin Subscribes

- Admin sees "Newsletter detected" on a website's detail page with a "Subscribe" button
- Clicking "Subscribe" creates a `newsletter_source` record and generates a unique ingest email address
- System uses **headless Chrome** to fill and submit the signup form with the generated email address
- Headless Chrome handles JS-only forms, captcha-adjacent flows, and varied form implementations

### 3. Per-Subscription Email Addresses

- Each subscription gets a unique address: `{uuid}@ingest.mntogether.org`
- Postmark inbound domain configured on `ingest.mntogether.org` (catch-all)
- Benefits:
  - **Kill switch**: disable one subscription without affecting others
  - **Spam isolation**: if an address leaks, burn just that one
  - **Automatic routing**: parse `to` address to identify which subscription sent the email
  - **Domain separation**: primary domain email reputation stays clean

### 4. Confirmation Flow

- Newsletter sends confirmation email to `{uuid}@ingest.mntogether.org`
- Postmark webhook receives it, routes by `to` address to the right subscription record
- Surfaced in admin panel as "Pending Confirmation" with the confirmation link extracted
- Admin reviews and clicks "Confirm" — system follows the confirmation link
- Subscription status transitions: `subscribing` → `pending_confirmation` → `active`

### 5. Newsletter Ingestion

- Each incoming newsletter email hits the Postmark webhook
- Email body stored as an `extraction_page`:
  - `site_url` = `newsletter:{source_id}` (grouping identifier)
  - `url` = `newsletter:{source_id}:{message_id}` (unique key)
  - `content` = email body (HTML converted to text/markdown)
  - `metadata` = `{ subject, sender, received_at }` (email-specific data)
- Fed through the existing post extraction pipeline — same 3-pass approach (narrative extraction, deduplication, agentic investigation)

### 6. Kill Switch

- Admin can deactivate any newsletter subscription
- System stops processing emails to that address
- Optional: configure Postmark to reject mail to deactivated addresses

## Key Decisions

- **Per-subscription email addresses over shared inbox**: Isolation, routing simplicity, independent kill switches
- **Subdomain (`ingest.mntogether.org`) over primary domain**: Protects main domain email reputation
- **Headless Chrome for form submission over simple POST**: Handles JS forms, varied implementations, honeypot fields — do it right
- **Store as extraction_pages over separate newsletter table**: Reuses entire extraction + search pipeline with zero changes
- **LLM-based newsletter detection over heuristics**: Already reading pages during org extraction, natural extension
- **Semi-automated confirmation (admin approves) over fully automated**: Avoids risk of auto-clicking unknown links

## What Needs to Change

### Extraction Library (minimal)
- No schema changes needed — `extraction_pages` is already generic
- Crawling domain's `find_by_domain()` needs a parallel query path for `newsletter:` site_urls (currently assumes `https://` prefix)
- Post extraction prompts may benefit from newsletter-aware context

### New Infrastructure
- Postmark inbound domain setup on `ingest.mntogether.org`
- Webhook endpoint to receive inbound emails
- Headless Chrome capability for form submission (Puppeteer/Playwright)
- `NewsletterIngestor` implementing the extraction library's `Ingestor` trait

### Database
- `newsletter_sources` table (class table inheritance extending `sources`):
  - `source_id` FK to sources
  - `ingest_email` (the generated `{uuid}@ingest.mntogether.org`)
  - `detected_signup_url` (where the newsletter form was found)
  - `subscription_status` (`subscribing`, `pending_confirmation`, `active`, `inactive`)
  - `confirmation_url` (extracted from confirmation email, for admin to approve)
- New source_type value: `'newsletter'` in the `sources` table

### Admin UI
- Newsletter detection indicator on website detail pages
- "Subscribe" button trigger
- Confirmation review panel (pending confirmations list)
- Newsletter source management (activate/deactivate)

## Open Questions

- Rate limiting: should we throttle how many newsletters we process per org per day?
- Retention: how long do we keep raw newsletter emails in extraction_pages?
- Unsubscribe: do we need automated unsubscribe capability, or is deactivating the ingest address sufficient?
- Multiple newsletters per org: some orgs have several newsletters — handle as separate sources or group?

## Next Steps

→ `/workflows:plan` for implementation details
