---
date: 2026-02-08
topic: home-footer-actions
---

# Home Footer Actions: Submit Link & Search Chat

## What We're Building

Two floating circular action buttons fixed at the bottom of the home page. Each opens a bottom sheet:

1. **Submit** (plus icon) — A simple link submission form. Users paste URLs from Instagram, Facebook, etc. The existing extraction pipeline processes the link automatically. A new `source` field distinguishes user-submitted links from crawler-discovered ones.

2. **Search** (magnifying glass) — An ephemeral mini chatroom powered by the public agent. Users describe what they need in natural language, and the agent searches the posts database using the existing `SearchPostsTool`. No persistence — conversation resets on close/refresh. Stale containers cleaned up by cron.

## Why This Approach

- **Bottom sheets over routes**: Keeps users on the home page. Quick, lightweight interactions without full page navigation.
- **Ephemeral chat over keyword search**: Natural language is more accessible. The agent can disambiguate, ask follow-ups, and surface relevant results conversationally.
- **Link-only submit**: Users post content on their own platforms (Instagram, Facebook) and share links here. MN Together extracts and curates — it's not a posting platform itself.

## Key Decisions

- **No localStorage persistence for chat**: Fresh conversation every time. Cron job cleans up stale containers on the backend.
- **Source tagging needed**: New field on posts to track `user_submitted` vs `crawler` origin.
- **Reuse existing infrastructure**: ChatPanel patterns, usePublicChatStream hook, Chat virtual object, submit_resource_link endpoint.
- **No new routes**: Everything happens in bottom sheets on the home page.

## Open Questions

- Exact cron schedule for cleaning up stale chat containers
- Whether source tagging goes on the `posts` table directly or on a related table

## Next Steps

- Plan implementation details
- Build bottom sheet component, submit sheet, search chat sheet
- Add source tagging to backend pipeline
