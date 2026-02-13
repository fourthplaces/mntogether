---
title: "Fix Admin Route Authorization Security Gaps"
type: fix
date: 2026-02-12
severity: HIGH
---

# Fix Admin Route Authorization Security Gaps

## Overview

A comprehensive security audit of the Restate service handlers revealed **16 handlers missing backend auth checks**. While the Next.js proxy requires a valid auth cookie for non-public paths, it does NOT verify admin status — meaning any authenticated (non-admin) user can invoke admin-only endpoints, including dangerous mutating/scheduled operations.

This is the second time this class of vulnerability has been identified (see `docs/plans/2026-02-02-fix-admin-authorization-security-gaps-plan.md`). The previous fix covered GraphQL resolvers; this audit covers the newer Restate service handlers.

## Problem Statement

The 3-layer auth architecture:

```
Next.js Middleware (UX guard, no crypto)
  → API Proxy (cookie existence check only)
    → Rust Backend (require_admin — WHERE REAL AUTH HAPPENS)
```

The proxy at `packages/web/app/api/restate/[...path]/route.ts` only checks that an `auth_token` cookie **exists**. It does NOT verify that the user is an admin. Any authenticated user (e.g., someone who submitted a post and logged in) can call handlers that should be admin-only.

## Findings

### Priority 1 — Data-Modifying Handlers (CRITICAL)

These handlers **mutate data or trigger expensive external operations** with zero auth checks:

| # | Service | Handler | File | Risk | Impact |
|---|---------|---------|------|------|--------|
| 1 | Posts | `expire_stale_posts` | `posts/restate/services/posts.rs:1386` | HIGH | Expires posts en masse |
| 2 | Members | `run_weekly_reset` | `member/restate/services/members.rs:119` | HIGH | Resets ALL notification counts |
| 3 | HeatMap | `compute_snapshot` | `heat_map/restate/services/heat_map.rs:66` | HIGH | Truncates + rewrites heat map data |
| 4 | Websites | `run_scheduled_scrape` | `website/restate/services/websites.rs:333` | HIGH | Triggers website crawling (API costs) |
| 5 | Websites | `run_scheduled_discovery` | `website/restate/services/websites.rs:540` | HIGH | Triggers discovery (API costs) |
| 6 | Sources | `run_scheduled_scrape` | `source/restate/services/sources.rs:691` | HIGH | Triggers source crawling |
| 7 | Sources | `run_scheduled_discovery` | `source/restate/services/sources.rs:882` | HIGH | Triggers source discovery |
| 8 | Organizations | `run_scheduled_extraction` | `organization/restate/services/organizations.rs:785` | HIGH | Triggers AI extraction (LLM costs) |
| 9 | SocialProfiles | `run_scheduled_scrape` | `social_profile/restate/services/social_profiles.rs:174` | MEDIUM-HIGH | Triggers Instagram scraping |
| 10 | Extraction | `submit_url` | `extraction/restate/services/extraction.rs:145` | HIGH | Submits arbitrary URLs (SSRF + costs) |

### Priority 2 — Data-Reading Handlers Exposing Admin Data (MEDIUM)

| # | Service | Handler | File | Risk | Impact |
|---|---------|---------|------|------|--------|
| 11 | Posts | `list` | `posts/restate/services/posts.rs:593` | HIGH | Exposes ALL posts (pending, rejected, archived) |
| 12 | Posts | `stats` | `posts/restate/services/posts.rs:1402` | MEDIUM | Exposes aggregate stats |
| 13 | Chats | `list_recent` | `chatrooms/restate/services/chats.rs:58` | HIGH | Exposes all chat containers |
| 14 | Extraction | `get_page` | `extraction/restate/services/extraction.rs:196` | MEDIUM | Exposes crawled page content |
| 15 | Extraction | `list_pages` | `extraction/restate/services/extraction.rs:213` | MEDIUM | Lists extracted pages |
| 16 | Extraction | `count_pages` | `extraction/restate/services/extraction.rs:234` | LOW | Counts pages by domain |

### Priority 3 — Defense-in-Depth (LOW)

| # | Layer | Issue | File | Risk |
|---|-------|-------|------|------|
| 17 | Proxy | No blocklist for internal/scheduled paths | `web/app/api/restate/[...path]/route.ts` | LOW |
| 18 | Middleware | Unvalidated `redirect` query parameter (open redirect) | `web/middleware.ts:55` | MEDIUM |

## Technical Considerations

### Scheduled Handler Auth Caveat

The `run_scheduled_*` handlers self-schedule via Restate's `send_after()`. When Restate invokes these internally, the call goes **directly to port 9080**, bypassing both the Next.js proxy and Restate ingress — so no auth headers are present.

**Two approaches:**

**(A) Proxy-level blocklist (Recommended):** Block these paths in the Next.js proxy so users can't reach them at all. Restate internal calls bypass the proxy, so they still work. This is the cleanest solution.

```typescript
// packages/web/app/api/restate/[...path]/route.ts
const INTERNAL_ONLY_PATHS = [
  "Posts/expire_stale_posts",
  "Websites/run_scheduled_scrape",
  "Websites/run_scheduled_discovery",
  "Sources/run_scheduled_scrape",
  "Sources/run_scheduled_discovery",
  "Organizations/run_scheduled_extraction",
  "Members/run_weekly_reset",
  "HeatMap/compute_snapshot",
  "SocialProfiles/run_scheduled_scrape",
];
```

**(B) Backend `require_admin`:** Add `require_admin()` to each handler. Simpler per-handler, but will cause self-scheduled calls to fail (no auth headers). Would need a fallback mechanism (e.g., skip auth if no headers present but called internally).

**Recommendation:** Use approach **(A)** for scheduled handlers + approach **(B)** for all other handlers (list, stats, submit_url, etc.).

### Open Redirect Fix

Validate the `redirect` parameter in `middleware.ts`:

```typescript
const redirectUrl = request.nextUrl.searchParams.get("redirect");
const isValidRedirect = redirectUrl
  && redirectUrl.startsWith("/admin/")
  && !redirectUrl.includes("//");
const destination = isValidRedirect ? redirectUrl : "/admin/dashboard";
```

Same validation needed in `LoginForm.tsx`.

## Acceptance Criteria

### Scheduled/Internal Handlers
- [x] Add `INTERNAL_ONLY_PATHS` blocklist to proxy route handler (`packages/web/app/api/restate/[...path]/route.ts`)
- [x] Return 403 for any path matching `INTERNAL_ONLY_PATHS` when called through the proxy
- [ ] Verify Restate self-scheduled calls still work (they bypass the proxy)

### Data-Reading Handlers
- [x] Add `require_admin` to `Posts/list` handler
- [x] Add `require_admin` to `Posts/stats` handler
- [x] Add `require_admin` to `Chats/list_recent` handler
- [x] Add `require_admin` to `Extraction/submit_url` handler
- [x] Add `require_admin` to `Extraction/get_page` handler
- [x] Add `require_admin` to `Extraction/list_pages` handler
- [x] Add `require_admin` to `Extraction/count_pages` handler
- [x] Decide: `HeatMap/get_latest` — intentionally public (per user decision)

### Defense-in-Depth
- [x] Validate `redirect` parameter in `middleware.ts` (must start with `/admin/`, no `//`)
- [x] Validate `redirect` parameter in `LoginForm.tsx`

### Verification
- [ ] All admin-only frontend pages still load correctly for admins
- [ ] Non-admin authenticated users cannot access admin data via API
- [ ] Scheduled jobs still execute on their normal cadence
- [x] `cargo build` succeeds with no new warnings

## Files to Modify

| File | Changes |
|------|---------|
| `packages/web/app/api/restate/[...path]/route.ts` | Add `INTERNAL_ONLY_PATHS` blocklist |
| `packages/server/src/domains/posts/restate/services/posts.rs` | Add `require_admin` to `list`, `stats` |
| `packages/server/src/domains/chatrooms/restate/services/chats.rs` | Add `require_admin` to `list_recent` |
| `packages/server/src/domains/extraction/restate/services/extraction.rs` | Add `require_admin` to `submit_url`, `get_page`, `list_pages`, `count_pages` |
| `packages/web/middleware.ts` | Validate `redirect` parameter |
| `packages/web/app/admin/(auth)/login/LoginForm.tsx` | Validate `redirect` parameter |

## Good Security Practices Already in Place

- httpOnly cookies with `secure: true` in production and `sameSite: "lax"`
- Full JWT signature verification at the Rust backend using `jsonwebtoken` crate
- Admin status baked into signed JWT (cannot be tampered with)
- Security headers: `X-Frame-Options: DENY`, `X-Content-Type-Options: nosniff`, etc.
- Phone number hashing before storage
- PII scrubbing for chat messages
- SSE endpoint validates JWT before allowing subscriptions
- `.env` properly gitignored
- SSRF protection via `ValidatedIngestor` wrapper
- Restate identity key support for production

## References

- Previous security plan: `docs/plans/2026-02-02-fix-admin-authorization-security-gaps-plan.md`
- Auth security docs: `docs/security/AUTHENTICATION_SECURITY.md`
- Auth guide: `docs/security/AUTHENTICATION_GUIDE.md`
- Dependency audit: `docs/security/SECURITY.md`
- JWT implementation: `packages/server/src/domains/auth/jwt.rs`
- `require_admin` helper: used extensively across handlers (see auth matrix above)
