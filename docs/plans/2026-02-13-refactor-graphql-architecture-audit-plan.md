---
title: "refactor: GraphQL architecture audit and cleanup"
type: refactor
date: 2026-02-13
---

# GraphQL Architecture Audit & Cleanup

## Overview

Audit and refactor the GraphQL layer across `packages/shared`, `packages/admin-app`, and `packages/web-app`. The goals: use GraphQL properly (nested types, single queries per page, fragments), remove redundant auth from the GraphQL layer (Restate handles auth), fix server inefficiencies (stale dataloaders, mutation double-fetch), and eliminate code duplication between apps.

## Problem Statement

The GraphQL migration from Restate is functionally complete — zero Restate calls from frontends. But the GraphQL layer was built as a 1:1 translation of the old RPC-style SWR hooks. Every `useRestate()` became a separate `useQuery()`. The result: detail pages fire 5-9 independent queries instead of leveraging GraphQL's nested type system. The server layer has redundant auth checks (Restate already validates JWT and enforces authorization), stale caches, and no dataloaders beyond Posts.

## Findings Summary

### Server Layer (`packages/shared/graphql/`)

| Issue | Severity | Files |
|-------|----------|-------|
| **Flat schema** — no nested type resolvers. Organization has no `sources`, `posts`, `notes`, `checklist` fields. Forces N separate root queries. | High | `schema.ts`, all resolvers |
| **Redundant auth in GraphQL** — all resolvers call `requireAdmin`/`requireAuth` before Restate calls, but Restate already validates JWT and enforces authorization. Double-ding on auth with no benefit. 4 resolvers also define local `requireAdmin` that throws plain `Error`. | High | all resolvers, `auth.ts`, `context.ts` |
| **Stale dataloader after mutations** — post mutations call `approve` then `postById.load()` without clearing cache | Medium | `post.ts`, `dataloaders/post.ts` |
| **Mutation double-fetch** — org/source/website mutations call action then re-call `get` (2 Restate round-trips) | Medium | `organization.ts`, `source.ts`, `website.ts` |
| **Only 3 dataloaders** — no loaders for Organization, Source, Website | Low | `dataloaders/` |
| **No enums** — all status/type fields are `String!` | Low | `schema.ts` |

### Frontend Layer (`packages/admin-app/`, `packages/web-app/`)

| Issue | Severity | Files |
|-------|----------|-------|
| **Multi-query detail pages** — org detail: 5 queries, source detail: 8, website detail: 9, post detail: 5 | High | All `[id]/page.tsx` files |
| **Duplicate query files** — `chat.ts`, `posts.ts`, `public.ts` are byte-identical between apps | Medium | 6 files total |
| **No fragments** — PostDetail fields copy-pasted 6+ times across queries | Medium | All `lib/graphql/*.ts` |
| **Public query fetches admin fields** — `PostDetailPublicQuery` in web-app requests `submittedBy`, `relevanceScore` | Low | `web-app/lib/graphql/public.ts` |

### What's Working Well

- All frontend data flows through GraphQL — zero direct Restate calls from UI
- urql provider setup is correct (per-request SSR exchange, document cache)
- Dataloaders are request-scoped (created in `createContext` per request)
- `snakeToCamel` transform handles Restate → GraphQL field mapping cleanly
- Codegen configured and working in both apps
- Mutation cache invalidation via `additionalTypenames` is correct pattern
- `RestateClient` already forwards cookies and maps Restate 401/403 → `GraphQLError` with proper codes — auth passthrough is already working

## Implementation Phases

### Phase 1: Remove all auth checks from GraphQL resolvers (quick win, no dependencies)

Restate already validates JWT (forwarded via `X-User-Token` header) and enforces authorization. The `RestateClient` already maps Restate's 401/403 responses to proper `GraphQLError` with `UNAUTHENTICATED`/`FORBIDDEN` codes. Auth checks in GraphQL resolvers are redundant — just pass cookies through and let Restate handle it.

- [x] `packages/shared/graphql/resolvers/post.ts` — remove all `requireAdmin(ctx)` calls
- [x] `packages/shared/graphql/resolvers/organization.ts` — remove all `requireAdmin(ctx)` calls
- [x] `packages/shared/graphql/resolvers/source.ts` — remove all `requireAdmin(ctx)` calls
- [x] `packages/shared/graphql/resolvers/website.ts` — remove all `requireAdmin(ctx)` calls
- [x] `packages/shared/graphql/resolvers/tag.ts` — remove all `requireAdmin(ctx)` calls
- [x] `packages/shared/graphql/resolvers/chat.ts` — remove all `requireAuth(ctx)` calls
- [x] `packages/shared/graphql/resolvers/sync.ts` — remove local `requireAdmin` function and all calls
- [x] `packages/shared/graphql/resolvers/note.ts` — remove local `requireAdmin` function and all calls
- [x] `packages/shared/graphql/resolvers/job.ts` — remove local `requireAdmin` function and all calls
- [x] `packages/shared/graphql/resolvers/search-query.ts` — remove local `requireAdmin` function and all calls
- [x] Delete `packages/shared/graphql/auth.ts` — no longer needed
- [x] Update `packages/shared/graphql/index.ts` — remove `requireAuth`, `requireAdmin` exports
- [x] Simplify `packages/shared/graphql/context.ts` — remove JWT decode logic (`decodeTokenClaims`), remove `user` field from `GraphQLContext` (cookie forwarding via `RestateClient` is sufficient)
- [x] Verify: zero `requireAdmin` or `requireAuth` references in `packages/shared/graphql/`

### Phase 2: Fix dataloader cache after mutations

Post mutations call `approve` then `postById.load()` — but the loader returns the stale cached value.

- [x] In `resolvers/post.ts`: call `ctx.loaders.postById.clear(id)` before `ctx.loaders.postById.load(id)` in all mutation resolvers that refetch
- [x] Verify: `approvePost`, `rejectPost`, `archivePost`, `reactivatePost`, `addPostTag`, `removePostTag`, `regeneratePost`, `regeneratePostTags`, `updatePostCapacity` — all 9 mutations + `approvePostInline`, `rejectPostInline` in website.ts

### Phase 3: Add nested type resolvers to schema + resolvers

This is the core architectural fix. Add field resolvers so detail pages can use single nested queries.

**Schema changes** (`packages/shared/graphql/schema.ts`):
- [x] Add to `Organization` type: `sources: [Source!]!`, `posts(limit: Int): PostConnection!`, `notes: [Note!]!`, `checklist: Checklist!`
- [x] Add to `Source` type: `pages: [ExtractionPage!]!`, `pageCount: Int!`, `assessment: Assessment`, `organization: Organization`
- [x] Add to `Website` type: `posts(limit: Int): PostConnection!`, `pages(limit: Int): [ExtractionPage!]!`, `pageCount: Int!`, `assessment: Assessment`, `organization: Organization`
- [x] Add to `Post` type: `organization: Organization` (resolve from `organizationId`)
- [x] Keep existing root queries as-is (non-breaking)

**Type resolvers** (`packages/shared/graphql/resolvers/`):

- [x] `organization.ts` — add `Organization` type resolver:
  ```
  Organization: {
    sources: (parent, _args, ctx) => ctx.restate.callService("Sources", "list_by_organization", { organization_id: parent.id }).then(r => r.sources),
    posts: (parent, args, ctx) => ctx.restate.callService("Posts", "list_by_organization", { organization_id: parent.id, limit: args.limit }),
    notes: (parent, _args, ctx) => ctx.restate.callService("Notes", "list_for_entity", { noteable_type: "organization", noteable_id: parent.id }).then(r => r.notes),
    checklist: (parent, _args, ctx) => ctx.restate.callService("Organizations", "get_checklist", { id: parent.id }),
  }
  ```

- [x] `source.ts` — add `Source` type resolver:
  ```
  Source: {
    pages: (parent, _args, ctx) => ctx.restate.callObject("Source", parent.id, "list_pages", {}).then(r => r.pages),
    pageCount: (parent, _args, ctx) => ctx.restate.callObject("Source", parent.id, "count_pages", {}).then(r => r.count),
    assessment: (parent, _args, ctx) => ctx.restate.callObject("Source", parent.id, "get_assessment", {}).then(r => r.assessment),
    organization: (parent, _args, ctx) => parent.organizationId ? ctx.restate.callService("Organizations", "get", { id: parent.organizationId }) : null,
  }
  ```

- [x] `website.ts` — add `Website` type resolver:
  ```
  Website: {
    posts: (parent, args, ctx) => ctx.restate.callService("Posts", "list", { source_type: "website", source_id: parent.id, first: args.limit ?? 100 }),
    pages: (parent, args, ctx) => ctx.restate.callService("Extraction", "list_pages", { domain: parent.domain, limit: args.limit ?? 50 }).then(r => r.pages),
    pageCount: (parent, _args, ctx) => ctx.restate.callService("Extraction", "count_pages", { domain: parent.domain }).then(r => r.count),
    assessment: (parent, _args, ctx) => ctx.restate.callObject("Website", parent.id, "get_assessment", {}).then(r => r.assessment),
    organization: (parent, _args, ctx) => parent.organizationId ? ctx.restate.callService("Organizations", "get", { id: parent.organizationId }) : null,
  }
  ```

- [x] `post.ts` — add `organization` to existing `Post` resolver:
  ```
  Post: {
    comments: ..., // existing
    organization: (parent, _args, ctx) => parent.organizationId ? ctx.restate.callService("Organizations", "get", { id: parent.organizationId }) : null,
  }
  ```

- [x] Run codegen + type-check in both apps

### Phase 4: Create GraphQL fragments

Define reusable fragments to eliminate field duplication across queries.

- [x] Create `packages/admin-app/lib/graphql/fragments.ts`:
  - `PostListFields` — fields used in list views (id, title, summary, status, postType, category, tags, etc.)
  - `PostDetailFields` — full fields for detail view (includes schedules, contacts, submittedBy, comments)
  - `PublicPostFields` — fields for public post cards
  - `OrganizationFields` — basic org fields
  - `SourceFields` — basic source fields
  - `NoteFields` — full note with linkedPosts

- [x] Refactor `packages/admin-app/lib/graphql/posts.ts` — use `PostListFields` and `PostDetailFields` fragments
- [x] Refactor `packages/admin-app/lib/graphql/organizations.ts` — use `OrganizationFields` fragment
- [x] Refactor `packages/admin-app/lib/graphql/sources.ts` — use `SourceFields` fragment

- [x] Create `packages/web-app/lib/graphql/fragments.ts`:
  - `PublicPostFields` — fields for public post cards
  - `PostDetailFields` — full fields (same as admin minus admin-only fields)

- [x] Refactor web-app query files to use fragments

### Phase 5: Consolidate to single queries per detail page

Replace multi-query patterns with single nested queries. This depends on Phase 3 (type resolvers).

**Admin org detail** (`packages/admin-app/lib/graphql/organizations.ts`):
- [x] Create `OrganizationDetailFullQuery`:
  ```graphql
  query OrganizationDetailFull($id: ID!) {
    organization(id: $id) {
      ...OrganizationFields
      sources { ...SourceFields }
      posts(limit: 100) { posts { ...PostListFields } totalCount }
      notes { ...NoteFields }
      checklist { items { key label checked checkedBy checkedAt } allChecked }
    }
  }
  ```
- [x] Update `admin-app/app/admin/(app)/organizations/[id]/page.tsx` — replace 5 `useQuery` calls with 1

**Admin source detail** (`packages/admin-app/lib/graphql/sources.ts`):
- [x] Create `SourceDetailFullQuery`:
  ```graphql
  query SourceDetailFull($id: ID!) {
    source(id: $id) {
      ...SourceFields
      pages { url content }
      pageCount
      assessment { id websiteId assessmentMarkdown confidenceScore }
      organization { id name }
    }
    organizations { id name }
  }
  ```
- [x] Update `admin-app/app/admin/(app)/sources/[id]/page.tsx` — replace 6 `useQuery` calls with 1 (keep workflow status queries separate since they poll)

**Admin website detail** (`packages/admin-app/lib/graphql/websites.ts`):
- [x] Create `WebsiteDetailFullQuery` (same pattern as source)
- [x] Update `admin-app/app/admin/(app)/websites/[id]/page.tsx` — replace 7 `useQuery` calls with 1 (keep workflow polls separate)

**Admin post detail** (`packages/admin-app/lib/graphql/posts.ts`):
- [x] Create `PostDetailFullQuery` (combines post + entityProposals + entityNotes in single query):
  ```graphql
  query PostDetailFull($id: ID!) {
    post(id: $id) {
      ...PostDetailFields
      organization { id name }
    }
    entityProposals(entityId: $id) { ... }
    entityNotes(noteableType: "post", noteableId: $id) { ...NoteFields }
  }
  ```
- [x] Update `admin-app/app/admin/(app)/posts/[id]/page.tsx` — replace 3 always-running `useQuery` calls with 1 (keep tag modal queries paused/deferred)

**Admin dashboard** (`packages/admin-app/lib/graphql/dashboard.ts`):
- [x] Create `DashboardQuery` using aliases:
  ```graphql
  query Dashboard {
    websites(limit: 1000) { websites { id domain status ... } totalCount hasNextPage }
    pendingPosts: posts(status: "pending_approval", limit: 1000) { posts { id status createdAt } totalCount }
    allPosts: posts(limit: 1000) { posts { id status createdAt } totalCount }
  }
  ```
- [x] Update `admin-app/app/admin/(app)/dashboard/page.tsx` — replace 3 `useQuery` calls with 1

### Phase 6: Clean up public query over-fetching

- [ ] Create separate `PublicPostDetailQuery` in web-app that excludes admin fields:
  - Remove: `relevanceScore`, `relevanceBreakdown`, `submittedBy`
  - Keep: `comments` (public feature)
  - Keep: `schedules`, `contacts`, `urgentNotes` (public info)

- [ ] Remove `admin-app/lib/graphql/public.ts` — admin-app no longer has public pages (they moved to web-app)

### Phase 7: Deduplicate shared operations

The web-app still has copies of admin-app query files that were copied during the public pages migration.

- [ ] Remove `web-app/lib/graphql/posts.ts` — web-app doesn't use admin post operations. It only uses `PublicPostsQuery` and `PostDetailPublicQuery` from `public.ts`, plus `SubmitResourceLinkMutation` and `AddCommentMutation`.
- [ ] Move needed post mutations (`SubmitResourceLinkMutation`, `AddCommentMutation`) into `web-app/lib/graphql/public.ts`
- [ ] Verify no web-app files import from `lib/graphql/posts.ts`
- [ ] Run codegen + type-check

### Phase 8: Add enums for key status fields

Replace `String!` with enums for compile-time validation.

- [ ] Add to schema:
  ```graphql
  enum PostStatus { DRAFT PENDING APPROVED REJECTED ARCHIVED }
  enum PostType { SERVICE OPPORTUNITY BUSINESS EVENT }
  enum OrganizationStatus { PENDING APPROVED REJECTED SUSPENDED }
  enum SourceStatus { PENDING APPROVED REJECTED }
  enum SourceType { WEBSITE INSTAGRAM FACEBOOK TIKTOK X }
  ```
- [ ] Update type definitions: `Post.status: PostStatus!`, `Post.postType: PostType`, etc.
- [ ] Update resolvers where enum values are passed to Restate (lowercase the enum values)
- [ ] Update frontend queries to use new enum types
- [ ] Run codegen — generated types will now use enums

## Verification

- [ ] `yarn workspace admin-app graphql-codegen` passes
- [ ] `yarn workspace web-app graphql-codegen` passes
- [ ] `npx tsc --noEmit --project packages/admin-app/tsconfig.json` passes
- [ ] `npx tsc --noEmit --project packages/web-app/tsconfig.json` passes
- [ ] Organization detail page: 1 primary query (was 5)
- [ ] Source detail page: 1 primary query + polling queries (was 8)
- [ ] Website detail page: 1 primary query + polling queries (was 9)
- [ ] Post detail page: 1-2 queries (was 5)
- [ ] Dashboard: 1 query (was 3)
- [ ] Zero `requireAdmin` or `requireAuth` calls in any resolver
- [ ] `auth.ts` deleted, no auth exports from `packages/shared/graphql/index.ts`
- [ ] No duplicate query files between apps

## Key Decisions

1. **Auth lives in Restate, not GraphQL** — GraphQL is a passthrough layer. Cookies are forwarded to Restate which validates JWT and enforces authorization. `RestateClient` maps 401/403 to `GraphQLError`. No `requireAdmin`/`requireAuth` in resolvers.
2. **Keep root queries** — adding nested type resolvers is additive, not breaking. Existing root queries (`organizationSources`, `sourcePages`, etc.) remain for any callers that need them.
3. **Dataloaders later** — true batch dataloaders require Restate batch endpoints which don't exist. Current deduplication-only loaders are fine for now. The bigger win is reducing total queries via nesting.
4. **Fragments per-app, not shared** — admin-app and web-app need different field sets. Fragments in each app's `lib/graphql/fragments.ts`.
5. **Workflow status queries stay separate** — `WorkflowStatusQuery` polls on an interval. Don't nest these into detail queries.
6. **Enums last** — enums are a schema-level change that touches everything. Do them after the structural refactor is stable.

## Dependencies & Risks

- **Risk**: Nested resolvers could make detail page slower if Restate calls aren't parallelized within GraphQL. **Mitigation**: GraphQL resolvers at the same depth level execute concurrently by default.
- **Risk**: Enum values might not match what Restate returns. **Mitigation**: Extract actual values from Restate/database before defining enums.
- **Risk**: Removing root queries like `organizationSources` could break if called elsewhere. **Mitigation**: Keep them, just add nested alternatives.

## References

- GraphQL schema: `packages/shared/graphql/schema.ts`
- All resolvers: `packages/shared/graphql/resolvers/*.ts`
- Dataloaders: `packages/shared/graphql/dataloaders/post.ts`
- Auth helpers (to be deleted): `packages/shared/graphql/auth.ts`
- Restate client (handles auth passthrough): `packages/shared/graphql/restate-client.ts`
- Admin operations: `packages/admin-app/lib/graphql/*.ts`
- Web operations: `packages/web-app/lib/graphql/*.ts`
- Multi-query pages: `packages/admin-app/app/admin/(app)/{organizations,sources,websites,posts}/[id]/page.tsx`
