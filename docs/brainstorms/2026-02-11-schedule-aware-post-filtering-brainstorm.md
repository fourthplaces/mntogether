---
date: 2026-02-11
topic: schedule-aware-post-filtering
---

# Schedule-Aware Post Filtering

## What We're Building

A unified approach to temporal relevance for posts. Posts with schedules whose occurrences have all passed should not appear in public queries. Events are just posts with schedules and a `post_type: event` tag — filtered via the existing tag system, no new routes or views needed.

The core principle: **the schedule IS the signal.** If a post has schedules, it's temporal. If it doesn't, it's evergreen.

## The Unified Design

### Rule

- **No schedules** → evergreen, always visible
- **Has schedules** → visible only if at least one schedule has a future occurrence

### Two Components

**1. Reusable SQL predicate (query-time filtering)**

Added to `find_public_filtered` and all public-facing queries:

```sql
AND (
  -- Evergreen: no schedules at all
  NOT EXISTS (
    SELECT 1 FROM schedules s
    WHERE s.schedulable_type = 'post' AND s.schedulable_id = p.id
  )
  -- OR at least one schedule is still active
  OR EXISTS (
    SELECT 1 FROM schedules s
    WHERE s.schedulable_type = 'post' AND s.schedulable_id = p.id
    AND (
      (s.rrule IS NULL AND COALESCE(s.dtend, s.dtstart) > NOW())
      OR (s.rrule IS NOT NULL AND (s.valid_to IS NULL OR s.valid_to >= CURRENT_DATE))
    )
  )
)
```

This is the primary correctness mechanism — no stale post ever leaks to users. Events are included in the default query alongside evergreen posts. To view only events, use the existing `post_type: event` tag filter via `find_public_filtered`.

**2. Daily sweep (data honesty)**

A Restate cron workflow that runs daily to keep `status` truthful:

```sql
UPDATE posts SET status = 'expired', updated_at = NOW()
WHERE status = 'active'
  AND EXISTS (
    SELECT 1 FROM schedules s
    WHERE s.schedulable_type = 'post' AND s.schedulable_id = posts.id
  )
  AND NOT EXISTS (
    SELECT 1 FROM schedules s
    WHERE s.schedulable_type = 'post' AND s.schedulable_id = posts.id
    AND (
      (s.rrule IS NULL AND COALESCE(s.dtend, s.dtstart) > NOW())
      OR (s.rrule IS NOT NULL AND (s.valid_to IS NULL OR s.valid_to >= CURRENT_DATE))
    )
  )
```

Zero urgency pressure — the query-time predicate is the safety net. The sweep just keeps admin dashboards and status counts honest.

## Why This Approach

- **Query-time predicate** = correctness guarantee, real-time, no lag
- **Daily sweep** = data hygiene, keeps `status` field truthful
- **Schedule as signal** = no post_type rules, no special flags, the data speaks for itself
- **SQL-only** = no application-level computation needed for filtering (rrule computation deferred to sort refinement later)

## Key Decisions

- **Schedule presence = temporal**: A schedule attached to a post makes it temporal. No schedule = evergreen.
- **Query-time is primary**: The sweep is a complement for data honesty, not the correctness mechanism.
- **No new routes or views**: Events are just posts with a `post_type: event` tag. Existing tag filter in `find_public_filtered` handles event-only views.
- **SQL-level dtstart sorting**: Good enough for now. Rust-side rrule next-occurrence sorting can be added later for recurring events.
- **Source disappearance already handled**: `Post::mark_disappeared()` sets `status = 'expired'` during source sync. No work needed there.

## Open Questions

- Verify index on `schedules(schedulable_type, schedulable_id)` covers the EXISTS subqueries efficiently.
- Should expired-by-schedule posts be visually distinct in admin views (vs. expired-by-source-sync)?

## Next Steps

→ `/workflows:plan` for implementation details
