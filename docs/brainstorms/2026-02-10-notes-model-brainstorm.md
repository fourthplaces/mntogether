---
date: 2026-02-10
topic: notes-model
---

# Notes Model — Attachable Alerts & Context for Entities

## What We're Building

A standalone "notes" model that acts as a graph-linked annotation system. Notes carry textual context and severity levels (info/notice/warn) and can be attached to any combination of entities — organizations, posts, websites, social profiles — via a polymorphic join table. Notes are created by admins manually or by the AI system automatically during crawling, and include source tracking so they can be refreshed against their origin to determine ongoing relevance.

## Why This Approach

We need a way to surface critical, time-sensitive information about organizations across the platform. For example, when an org posts on their website and Instagram that they're pausing donations, that context needs to propagate to any posts/listings associated with that org on mntogether.org. A polymorphic graph-linked model (rather than notes "owned" by a single entity) gives maximum flexibility — a single note can be linked to an org AND its specific posts simultaneously.

This follows the existing `taggables` polymorphic pattern already in the codebase.

## Key Decisions

- **Notes are standalone entities**: Not owned by any single model. Linked via `noteables` join table.
- **Polymorphic linking (like taggables)**: `noteable_type` + `noteable_id` pattern, allowing notes to attach to organizations, posts, websites, social profiles.
- **Source tracking for refresh**: `source_id`, `source_type`, and `source_url` let the system re-check the origin content and expire notes when no longer relevant.
- **`expired_at` instead of delete**: When a source no longer matches, set `expired_at` rather than deleting — preserves history.
- **Severity levels**: `info` (general context), `notice` (worth knowing), `warn` (action needed / surface prominently).
- **`is_public` toggle**: Notes can optionally be surfaced to public users.
- **`created_by` field**: Tracks whether admin or system created the note.
- **AI as primary creator**: The crawling pipeline will be the heaviest note creator — detecting significant org updates and auto-linking to relevant posts.

## Schema Design

### `notes` table

| Field | Type | Notes |
|-------|------|-------|
| id | UUID PK | |
| content | TEXT NOT NULL | The note text |
| severity | TEXT NOT NULL DEFAULT 'info' | `info` / `notice` / `warn` |
| source_url | TEXT | Direct link to where info came from |
| source_id | UUID | FK to the content that generated this (page, social post, etc.) |
| source_type | TEXT | `instagram` / `website` / `facebook` / `manual` |
| is_public | BOOLEAN NOT NULL DEFAULT false | Whether to surface publicly |
| created_by | TEXT NOT NULL DEFAULT 'system' | `admin` / `system` |
| expired_at | TIMESTAMPTZ | Set when source no longer matches |
| created_at | TIMESTAMPTZ NOT NULL | |
| updated_at | TIMESTAMPTZ NOT NULL | |

### `noteables` join table

| Field | Type | Notes |
|-------|------|-------|
| id | UUID PK | |
| note_id | UUID FK → notes(id) ON DELETE CASCADE | |
| noteable_type | TEXT NOT NULL | `organization` / `post` / `website` / `social_profile` |
| noteable_id | UUID NOT NULL | ID of the linked entity |
| added_at | TIMESTAMPTZ NOT NULL | |

Unique constraint on `(note_id, noteable_type, noteable_id)`.
Indexes on `(noteable_type, noteable_id)` and `note_id`.

## Example Flow

1. Crawl org website → detect "pausing physical donations" language → create note (severity: `warn`, source_type: `website`, source_id: page_id)
2. Crawl org Instagram → detect "stop donating" post → create note (severity: `warn`, source_type: `instagram`, source_id: social post reference)
3. Both notes linked to the Organization via `noteables`
4. System cross-references org's posts on mntogether → finds "volunteer at Some Nonprofit" listing → auto-links warn notes to that post
5. Public-facing post now surfaces the warning context
6. Later, org resumes donations → refresh cycle re-checks source → sets `expired_at` on the notes

## Open Questions

- Should there be a `title` field on notes, or is `content` + `severity` sufficient?
- When displaying expired notes, should they be hidden entirely or shown as "resolved"?
- For AI auto-linking: what confidence threshold before automatically linking a note to a post?
- Should notes support threading/replies (admin responds to a system-generated note)?

## Next Steps

→ `/workflows:plan` for implementation details (migration, Rust model, activities, admin UI)
