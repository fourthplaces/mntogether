# Abuse Reporting

## Why This Exists

MN Together publishes community-sourced content: posts from scraped sources, admin-authored stories, and org-submitted listings. Some of this content will be wrong, outdated, misleading, or harmful. Readers need a way to flag problems, and editors need a way to act on those flags.

Without reporting, the only feedback loop is "someone emails Tim." That doesn't scale and it means bad content stays up until an editor happens to notice it.

## What It Does

**For readers** (public site): a "Report" button on any post. Tap it, pick a reason, optionally leave an email, submit. No account required.

**For editors** (admin CMS): a reports queue showing flagged posts. Each report shows the reason, when it was filed, and the post it targets. Editors can resolve (take action + leave notes) or dismiss (not actionable + leave notes).

## Report Categories

| Category | When to use |
|----------|-------------|
| `inaccurate` | Information is wrong or outdated |
| `misleading` | Technically true but presented deceptively |
| `duplicate` | Same content as another post |
| `spam` | Promotional, irrelevant, or bot content |
| `offensive` | Hateful, discriminatory, or harmful language |
| `scam` | Fraudulent services, phishing, financial exploitation |
| `safety` | Content that could endanger someone |
| `other` | Doesn't fit the above — free-text reason required |

## Report Lifecycle

```
Reader submits report
        │
        ▼
   ┌─────────┐
   │ pending  │  ← appears in admin queue
   └────┬─────┘
        │
   editor reviews
        │
   ┌────┴────┐
   │         │
   ▼         ▼
resolved  dismissed
(action    (no action
 taken)     needed)
```

Both resolution paths require notes explaining the decision. Resolved reports also record what action was taken (e.g., "archived post", "edited misleading claim", "contacted organization").

## Current State

The Rust backend is mostly built. The database and frontend are not.

| Layer | Status | Location |
|-------|--------|----------|
| DB migration | Missing | Needs `post_reports` table |
| Rust model | Done | `domains/posts/models/post_report.rs` |
| Rust data types | Done | `domains/posts/data/post_report.rs` |
| Activities | Done | `domains/posts/activities/reports.rs` |
| HTTP handlers | Done | `api/routes/posts.rs` (5 endpoints) |
| GraphQL | Missing | Not exposed via GraphQL yet |
| Admin UI | Missing | No reports page in admin-app |
| Public UI | Missing | No report button in web-app |
| Tests | Missing | No integration tests |

### Existing HTTP Endpoints

```
POST /Post/{id}/report          → submit report (public, no auth required)
POST /Posts/list_reports         → list reports (admin)
POST /Post/{id}/get_reports     → reports for a specific post (admin)
POST /Post/{id}/resolve_report  → resolve with action + notes (admin)
POST /Post/{id}/dismiss_report  → dismiss with notes (admin)
```

### Data Model (from Rust structs)

```
PostReportRecord
├── id (UUID)
├── post_id (UUID FK → posts)
├── reported_by (UUID FK → members, nullable for anonymous)
├── reporter_email (optional, for anonymous follow-up)
├── reason (TEXT, free-form)
├── category (TEXT, from categories above)
├── status: pending | resolved | dismissed
├── resolved_by (UUID FK → members, nullable)
├── resolved_at (TIMESTAMPTZ, nullable)
├── resolution_notes (TEXT, nullable)
├── action_taken (TEXT, nullable)
├── created_at
└── updated_at
```

## What Needs to Be Built

### 1. Database migration

Create `post_reports` table matching the Rust struct above. Add indexes on `(post_id)`, `(status)`, and `(created_at)`. Consider a view joining post title/status for the admin list query.

### 2. Admin reports page

Route: `/admin/reports`

A queue-style page:
- Default view: pending reports, newest first
- Each row: post title, report category, reason excerpt, time filed
- Click to expand: full reason, reporter email (if provided), post link
- Actions: Resolve (text input for notes + action taken) or Dismiss (text input for notes)
- Filter: pending / resolved / dismissed / all
- Badge count of pending reports in the admin nav

### 3. Report button on public post pages

On the web-app post detail page:
- Small "Report" link or flag icon, low-profile placement
- Opens a modal: category select, reason textarea, optional email field
- Submits to `POST /Post/{id}/report`
- Confirmation message after submit

### 4. Report indicator on admin post detail

On the admin post detail page (right sidebar):
- If the post has pending reports, show a count badge
- Link to the filtered reports view for that post

### 5. Integration tests

Test through the HTTP API:
- Anonymous report submission (no auth)
- Authenticated report submission
- Admin list/resolve/dismiss flow
- Verify status transitions and timestamps
