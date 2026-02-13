---
title: "refactor: Migrate admin-app from Restate to GraphQL"
type: refactor
date: 2026-02-13
---

# Migrate admin-app from Restate to GraphQL

## Overview

Eliminate ALL direct Restate calls from `packages/admin-app`. Every data fetch and mutation goes through GraphQL (`packages/shared`). Zero Restate knowledge in frontend code.

```
admin-app pages → urql hooks (typed) → /api/graphql → GraphQL Yoga → RestateClient → Restate → Rust
```

## Problem Statement

admin-app currently makes ~130 direct Restate calls via SWR hooks and mutation helpers (`useRestate`, `callService`, `callObject`). These are stringly-typed, manually cache-invalidated, and leak backend implementation details into the frontend. The GraphQL layer in `packages/shared` only covers the Posts domain (~7 operations). The remaining 12+ domains have zero GraphQL coverage.

The SpecFlow analysis identified 16 distinct user flows and ~130 total operations (vs the initial estimate of ~95), including public pages, workflow status polling, and comments that were not in the original count.

## Proposed Solution

Expand the shared GraphQL schema domain-by-domain, then migrate each admin-app page from SWR+Restate to urql+GraphQL. Work in phases ordered by dependency (shared types first, then domains with most downstream usage).

## Technical Approach

### What Already Exists (packages/shared)

- `schema.ts` — inline SDL with Post types, queries, mutations
- `resolvers/post.ts` — publicPosts, publicFilters, post, posts, trackPostView, trackPostClick, Post.comments
- `dataloaders/post.ts` — postById, commentsByPostId
- `context.ts` — createContext with auth token parsing, RestateClient, loaders
- `restate-client.ts` — RestateClient with callService/callObject, snakeToCamel transform, error mapping
- `auth.ts` — requireAuth, requireAdmin helpers

### What Needs Building

**16 user flows, ~130 operations total (per SpecFlow analysis):**

| Domain | Queries | Mutations | Files to Migrate |
|--------|---------|-----------|-----------------|
| Organizations | 4 (list, detail, checklist, publicList/publicGet) | 13 (update, delete, approve, reject, suspend, setStatus, toggleChecklist, extractPosts, cleanUpPosts, runCurator, crawlAllSources, regenerate, removeAllPosts, removeAllNotes) | `organizations/page.tsx`, `organizations/[id]/page.tsx` |
| Posts (remaining) | 2 (stats, list with filters) | 12 (approve, reject, archive, delete, reactivate, addTag, removeTag, regenerate, regenerateTags, updateCapacity, batchScorePosts, rewriteNarratives) | `posts/page.tsx`, `posts/[id]/page.tsx` |
| Sources | 6 (list, detail, snapshots, listByOrg, listPages, countPages, getAssessment) | 12 (approve, reject, crawl, generateAssessment, extractOrg, assignOrg, unassignOrg, regeneratePosts, deduplicatePosts, submitWebsite, createSocial, searchByContent, lightCrawlAll) | `sources/page.tsx`, `sources/[id]/page.tsx`, `sources/[id]/snapshots/page.tsx` |
| Websites | 4 (list, detail, snapshots, assessment) | 8 (approve, reject, crawl, generateAssessment, assignOrg, unassignOrg, deduplicatePosts, extractOrganization, submitWebsite) | `websites/page.tsx`, `websites/[id]/page.tsx`, `websites/[id]/snapshots/page.tsx` |
| Tags | 2 (listKinds, listTags) | 6 (createKind, updateKind, deleteKind, createTag, updateTag, deleteTag) | `tags/page.tsx` |
| Proposals/Sync | 3 (listBatches, listProposals, listEntityProposals) | 5 (approveProposal, rejectProposal, approveBatch, rejectBatch, refineProposal) | `proposals/page.tsx`, embedded in `posts/[id]/page.tsx` |
| Notes | 1 (listForEntity) | 6 (create, update, delete, unlink, generateNotes, attachNotes) | embedded in `organizations/[id]/page.tsx` |
| Search Queries | 1 (list) | 5 (create, update, toggle, delete, runDiscovery) | `search-queries/page.tsx` |
| Jobs | 1 (list) | 0 | `jobs/page.tsx` |
| Dashboard | 3 (websiteStats, postStats, pendingPosts) | 0 | `dashboard/page.tsx` |
| Extraction | 3 (listPages, countPages, getPage) | 0 | snapshots pages (websites + sources) |
| Workflow Polling | 3 (RegeneratePosts, RegenerateSocialPosts, DeduplicatePosts getStatus) | 3 (workflow run invocations) | website detail, source detail |
| Comments | 1 (getComments) | 1 (addComment) | `components/public/CommentsSection.tsx` |
| Public Pages | 4 (publicList, publicGet, publicFilters for orgs + posts) | 2 (submitResourceLink, trackView/trackClick) | `(public)/**` pages, `SubmitSheet.tsx` |
| Social Profiles | 0 (loaded via org detail) | 1 (createSocial) | embedded in `organizations/[id]/page.tsx` |

**Excluded from migration (stays as-is):**
- **Auth** (send_otp, verify_otp, logout) — server actions, needs cookie control
- **SSE/Chat streaming** — stays with SSE proxy for now, deferred to future phase
- **Chat mutations** (sendMessage, listChats, getMessages, createChat) — deferred with SSE

### Key Architectural Decisions

1. **Auth stays as server actions** — OTP flow needs `cookies().set()` which GraphQL can't do cleanly
2. **Chat/SSE deferred** — SSE proxy stays until GraphQL subscriptions are evaluated
3. **Schema is inline SDL** in `schema.ts` — matches existing pattern, not .graphql files
4. **requireAdmin on all admin mutations/queries** — prevents the auth gaps found in the security audit
5. **urql document cache** with `additionalTypenames` for refetch after mutations — simple, no normalized cache
6. **One resolver file per domain** — `resolvers/organization.ts`, `resolvers/source.ts`, etc.
7. **Dataloaders for entities referenced across domains** — organizationById, sourceById, websiteById, tagById
8. **Long-running ops return immediately** — mutations like `regeneratePosts`, `crawlWebsite` return a workflowId; clients poll via `workflowStatus(id: ID!)` query
9. **Incremental migration** — both SWR/Restate and urql/GraphQL coexist during migration; domains migrated one at a time
10. **Public pages included** — `(public)/*` pages in admin-app also migrate to GraphQL (some already have GraphQL equivalents)

### Auth Mapping (per SpecFlow analysis)

| Auth Level | Operations |
|-----------|-----------|
| **No auth (public)** | publicPosts, publicFilters, publicPost, publicOrganizations, publicOrganization, trackPostView, trackPostClick, submitResourceLink, addComment, getComments |
| **requireAuth** | Chat create/sendMessage (deferred) |
| **requireAdmin** | All other queries and mutations (organizations list/detail, posts list/stats, all CRUD mutations, all workflow triggers, etc.) |

### Cache Invalidation Strategy

- Mutations return updated entities where possible → urql auto-updates document cache
- For cross-entity invalidation (e.g., approve post → org post list changes), use `additionalTypenames: ["Post", "Organization"]`
- The `invalidateService("X")` pattern maps to `additionalTypenames: ["TypeName"]` which refetches all queries returning that type
- Conditional queries (current `useRestate(null, ...)` pattern) map to urql's `pause: true` option

### Codegen Configuration

The admin-app's `codegen.ts` points to `../shared/graphql/typeDefs/**/*.graphql` but the schema is inline in TypeScript. Two options:
- **Option A**: Extract SDL to `.graphql` files under `packages/shared/graphql/typeDefs/` (one per domain)
- **Option B**: Point codegen at the introspection endpoint (`http://localhost:3000/api/graphql`)
- **Chosen: Option A** — extract to `.graphql` files for IDE support and codegen without running a server

## Implementation Phases

### Phase 1: Wire admin-app GraphQL route + foundation

Fix the stub. Wire admin-app to actually use the shared GraphQL schema.

1. Update `packages/admin-app/app/api/graphql/route.ts` to import from `@mntogether/shared` (same pattern as web-app/search-app)
2. Update `packages/admin-app/package.json` to use `@mntogether/shared` as `file:../shared` dependency
3. Verify the existing Post queries work through admin-app's GraphQL endpoint
4. Run `yarn codegen` in admin-app to verify generated types

**Files changed:**
- `packages/admin-app/app/api/graphql/route.ts`
- `packages/admin-app/package.json`

### Phase 2: Tags domain (simplest CRUD, no dependencies)

Tags have clean CRUD with no foreign key relationships to other domains. Good warmup.

**Schema additions:**
```graphql
extend type Query {
  tagKinds: [TagKind!]!
  tags: [Tag!]!
}

extend type Mutation {
  createTagKind(slug: String!, displayName: String!, description: String, required: Boolean, isPublic: Boolean, allowedResourceTypes: [String!]): TagKind!
  updateTagKind(id: ID!, displayName: String, description: String, required: Boolean, isPublic: Boolean): TagKind!
  deleteTagKind(id: ID!): Boolean!
  createTag(kindId: ID!, value: String!, displayName: String, color: String, description: String, emoji: String): Tag!
  updateTag(id: ID!, displayName: String, color: String, description: String, emoji: String): Tag!
  deleteTag(id: ID!): Boolean!
}

type TagKind {
  id: ID!
  slug: String!
  displayName: String!
  description: String
  allowedResourceTypes: [String!]!
  required: Boolean!
  isPublic: Boolean!
  tagCount: Int!
}
```

**Files:**
- `packages/shared/graphql/resolvers/tag.ts` (new)
- `packages/shared/graphql/schema.ts` (extend)
- `packages/admin-app/app/admin/(app)/tags/page.tsx` (rewrite imports)

### Phase 3: Post mutations + public interactions (extend existing domain)

Post queries already exist. Add the missing admin mutations, public submit, and comments.

**Schema additions:**
```graphql
extend type Query {
  postStats: PostStats!
}

extend type Mutation {
  approvePost(id: ID!): Post!
  rejectPost(id: ID!, reason: String): Post!
  archivePost(id: ID!): Post!
  deletePost(id: ID!): Boolean!
  reactivatePost(id: ID!): Post!
  addPostTag(postId: ID!, tagId: ID!): Post!
  removePostTag(postId: ID!, tagId: ID!): Post!
  regeneratePost(id: ID!): Post!
  regeneratePostTags(id: ID!): Post!
  updatePostCapacity(id: ID!, capacityStatus: String!): Post!
  batchScorePosts: Boolean!
  rewriteNarratives(id: ID!): Post!
  submitResourceLink(url: String!, title: String, description: String): Post!
  addComment(postId: ID!, content: String!, authorName: String): Comment!
}

type PostStats {
  total: Int!
  services: Int!
  opportunities: Int!
  businesses: Int!
  userSubmitted: Int!
  scraped: Int!
}
```

**Files:**
- `packages/shared/graphql/resolvers/post.ts` (extend)
- `packages/shared/graphql/schema.ts` (extend)
- `packages/admin-app/app/admin/(app)/posts/page.tsx` (rewrite)
- `packages/admin-app/app/admin/(app)/posts/[id]/page.tsx` (rewrite)
- `packages/admin-app/components/public/CommentsSection.tsx` (rewrite)
- `packages/admin-app/app/(public)/submit/page.tsx` (rewrite)
- `packages/admin-app/components/public/SubmitSheet.tsx` (rewrite)

### Phase 4: Organizations domain (largest, most complex)

Organizations have the most calls (~25+) and touch notes, sources, social profiles, checklists, and posts.

**Schema additions:**
```graphql
extend type Query {
  organizations: [Organization!]!
  organization(id: ID!): OrganizationDetail
  publicOrganizations(limit: Int, offset: Int): [Organization!]!
  publicOrganization(id: ID!): OrganizationDetail
}

extend type Mutation {
  updateOrganization(id: ID!, name: String, description: String): Organization!
  deleteOrganization(id: ID!): Boolean!
  approveOrganization(id: ID!): Organization!
  rejectOrganization(id: ID!, reason: String!): Organization!
  suspendOrganization(id: ID!, reason: String!): Organization!
  setOrganizationStatus(id: ID!, status: String!, reason: String): Organization!
  toggleChecklistItem(organizationId: ID!, key: String!): Checklist!
  extractOrgPosts(id: ID!): Boolean!
  cleanUpOrgPosts(id: ID!): Boolean!
  runCurator(id: ID!): Boolean!
  crawlAllOrgSources(id: ID!): Boolean!
  regenerateOrganization(id: ID!): Boolean!
  removeAllOrgPosts(id: ID!): Boolean!
  removeAllOrgNotes(id: ID!): Boolean!
  createNote(organizationId: ID!, content: String!, severity: String, ctaText: String, isPublic: Boolean, sourceUrl: String): Note!
  updateNote(id: ID!, content: String, severity: String, ctaText: String, isPublic: Boolean, expiredAt: String): Note!
  deleteNote(id: ID!): Boolean!
  unlinkNote(noteId: ID!, postId: ID!): Boolean!
  generateNotes(organizationId: ID!): Boolean!
  attachNotes(organizationId: ID!): Boolean!
  createSocialProfile(organizationId: ID!, platform: String!, handle: String!, url: String): SocialProfile!
}

type Organization { id: ID!, name: String!, description: String, status: String!, websiteCount: Int!, socialProfileCount: Int!, snapshotCount: Int!, createdAt: String!, updatedAt: String! }
type OrganizationDetail { id: ID!, name: String!, description: String, posts: [PublicPost!]!, sources: [Source!]!, socialProfiles: [SocialProfile!]!, notes: [Note!]!, checklist: Checklist! }
type Checklist { items: [ChecklistItem!]!, allChecked: Boolean! }
type ChecklistItem { key: String!, label: String!, checked: Boolean!, checkedBy: String, checkedAt: String }
type Note { id: ID!, content: String!, ctaText: String, severity: String!, sourceUrl: String, sourceId: String, sourceType: String, isPublic: Boolean!, createdBy: String!, expiredAt: String, createdAt: String!, updatedAt: String!, linkedPosts: [LinkedPost!] }
type LinkedPost { id: ID!, title: String! }
type SocialProfile { id: ID!, organizationId: ID!, platform: String!, handle: String!, url: String, scrapeFrequencyHours: Int!, lastScrapedAt: String, active: Boolean!, createdAt: String! }
```

**Files:**
- `packages/shared/graphql/resolvers/organization.ts` (new)
- `packages/shared/graphql/dataloaders/organization.ts` (new)
- `packages/shared/graphql/schema.ts` (extend)
- `packages/admin-app/app/admin/(app)/organizations/page.tsx` (rewrite)
- `packages/admin-app/app/admin/(app)/organizations/[id]/page.tsx` (rewrite)

### Phase 5: Sources domain

**Schema additions:**
```graphql
extend type Query {
  sources(limit: Int, offset: Int, search: String): SourceConnection!
  source(id: ID!): Source
  sourceSnapshots(sourceId: ID!): [ExtractionPage!]!
  sourceSnapshotCount(sourceId: ID!): Int!
  sourceAssessment(sourceId: ID!): Assessment
  sourcesByOrganization(organizationId: ID!): [Source!]!
  searchSourcesByContent(query: String!, limit: Int): [Source!]!
}

extend type Mutation {
  approveSource(id: ID!): Source!
  rejectSource(id: ID!, reason: String): Source!
  crawlSource(id: ID!): Boolean!
  generateSourceAssessment(id: ID!): Boolean!
  extractSourceOrganization(id: ID!): Boolean!
  assignSourceOrganization(sourceId: ID!, organizationId: ID!): Source!
  unassignSourceOrganization(sourceId: ID!): Source!
  regenerateSourcePosts(sourceId: ID!): Boolean!
  deduplicateSourcePosts(sourceId: ID!): Boolean!
  submitSourceWebsite(url: String!): Source!
  createSocialSource(organizationId: ID!, platform: String!, handle: String!, url: String): Source!
  lightCrawlAllSources: Boolean!
}

type Source { id: ID!, sourceType: String!, identifier: String!, url: String, status: String!, active: Boolean!, organizationId: ID, organizationName: String, scrapeFrequencyHours: Int!, lastScrapedAt: String, postCount: Int, snapshotCount: Int, createdAt: String!, updatedAt: String! }
type SourceConnection { sources: [Source!]!, totalCount: Int!, hasNextPage: Boolean!, hasPreviousPage: Boolean! }
type ExtractionPage { url: String!, content: String }
```

**Files:**
- `packages/shared/graphql/resolvers/source.ts` (new)
- `packages/shared/graphql/schema.ts` (extend)
- `packages/admin-app/app/admin/(app)/sources/page.tsx` (rewrite)
- `packages/admin-app/app/admin/(app)/sources/[id]/page.tsx` (rewrite)
- `packages/admin-app/app/admin/(app)/sources/[id]/snapshots/page.tsx` (rewrite)

### Phase 6: Websites domain

**Schema additions:**
```graphql
extend type Query {
  websites(limit: Int, offset: Int): WebsiteConnection!
  website(id: ID!): WebsiteDetail
  websiteSnapshots(websiteId: ID!): [ExtractionPage!]!
  websiteSnapshotCount(websiteId: ID!): Int!
  websiteAssessment(websiteId: ID!): Assessment
}

extend type Mutation {
  submitWebsite(url: String!): Boolean!
  approveWebsite(id: ID!): Boolean!
  rejectWebsite(id: ID!, reason: String): Boolean!
  crawlWebsite(id: ID!): Boolean!
  generateWebsiteAssessment(id: ID!): Boolean!
  assignWebsiteOrganization(websiteId: ID!, organizationId: ID!): Boolean!
  unassignWebsiteOrganization(websiteId: ID!): Boolean!
  deduplicateWebsitePosts(websiteId: ID!): Boolean!
  extractWebsiteOrganization(websiteId: ID!): Boolean!
  regenerateWebsitePosts(websiteId: ID!): Boolean!
}

type WebsiteDetail { id: ID!, domain: String!, status: String!, submittedBy: String, submitterType: String!, lastScrapedAt: String, snapshotsCount: Int!, listingsCount: Int!, createdAt: String!, snapshots: [SnapshotResult!]!, listings: [Post!]!, organizationId: ID }
type WebsiteConnection { websites: [Website!]!, totalCount: Int!, hasNextPage: Boolean! }
type Website { id: ID!, domain: String!, status: String!, active: Boolean!, createdAt: String, crawlCount: Int, postCount: Int, lastCrawledAt: String, organizationId: ID }
type Assessment { id: ID!, websiteId: ID!, assessmentMarkdown: String!, confidenceScore: Float }
type SnapshotResult { url: String!, siteUrl: String!, title: String, content: String!, fetchedAt: String!, listingsCount: Int! }
```

**Files:**
- `packages/shared/graphql/resolvers/website.ts` (new)
- `packages/shared/graphql/schema.ts` (extend)
- `packages/admin-app/app/admin/(app)/websites/page.tsx` (rewrite)
- `packages/admin-app/app/admin/(app)/websites/[id]/page.tsx` (rewrite)
- `packages/admin-app/app/admin/(app)/websites/[id]/snapshots/page.tsx` (rewrite)

### Phase 7: Proposals/Sync domain

**Schema additions:**
```graphql
extend type Query {
  syncBatches: [SyncBatch!]!
  syncProposals(batchId: ID!): [SyncProposal!]!
  entityProposals(entityId: ID!): [SyncProposal!]!
}

extend type Mutation {
  approveSyncProposal(proposalId: ID!): Boolean!
  rejectSyncProposal(proposalId: ID!): Boolean!
  approveSyncBatch(batchId: ID!): Boolean!
  rejectSyncBatch(batchId: ID!): Boolean!
  refineProposal(proposalId: ID!, instructions: String): Boolean!
}

type SyncBatch { id: ID!, resourceType: String!, sourceId: ID, sourceName: String, status: String!, summary: String, proposalCount: Int!, approvedCount: Int!, rejectedCount: Int!, createdAt: String!, reviewedAt: String }
type SyncProposal { id: ID!, batchId: ID!, operation: String!, status: String!, entityType: String!, draftEntityId: ID, targetEntityId: ID, reason: String, reviewedBy: String, reviewedAt: String, createdAt: String!, draftTitle: String, targetTitle: String, mergeSourceIds: [String!]!, mergeSourceTitles: [String!]!, relevanceScore: Float, curatorReasoning: String, confidence: String, sourceUrls: [String!], revisionCount: Int }
```

**Files:**
- `packages/shared/graphql/resolvers/sync.ts` (new)
- `packages/shared/graphql/schema.ts` (extend)
- `packages/admin-app/app/admin/(app)/proposals/page.tsx` (rewrite)

### Phase 8: Search Queries, Jobs, Dashboard, Workflow Polling (small domains)

**Schema additions:**
```graphql
extend type Query {
  searchQueries: [SearchQuery!]!
  jobs: [Job!]!
  dashboardStats: DashboardStats!
  workflowStatus(id: ID!): WorkflowStatus
}

extend type Mutation {
  createSearchQuery(queryText: String!, isActive: Boolean): SearchQuery!
  updateSearchQuery(id: ID!, queryText: String, sortOrder: Int): SearchQuery!
  toggleSearchQuery(id: ID!): SearchQuery!
  deleteSearchQuery(id: ID!): Boolean!
  runScheduledDiscovery: Boolean!
}

type SearchQuery { id: ID!, queryText: String!, isActive: Boolean!, sortOrder: Int! }
type Job { id: ID!, workflowName: String!, workflowKey: String!, status: String!, progress: String, createdAt: String, modifiedAt: String, completedAt: String, completionResult: String, websiteDomain: String, websiteId: ID }
type DashboardStats { websiteStats: WebsiteConnection!, pendingPosts: PostConnection!, allPosts: PostConnection! }
type WorkflowStatus { status: String!, message: String, completedAt: String }
```

**Workflow status polling:** Long-running mutations (regeneratePosts, crawlWebsite, deduplicatePosts, etc.) return a `workflowId`. Pages poll `workflowStatus(id)` via urql's `useQuery` with `pollInterval` or `requestPolicy: 'network-only'` on a timer. This replaces the current `callObject("*Workflow", id, "get_status")` + `setInterval` pattern.

**Files:**
- `packages/shared/graphql/resolvers/searchQuery.ts` (new)
- `packages/shared/graphql/resolvers/job.ts` (new)
- `packages/shared/graphql/resolvers/workflow.ts` (new)
- `packages/shared/graphql/schema.ts` (extend)
- `packages/admin-app/app/admin/(app)/search-queries/page.tsx` (rewrite)
- `packages/admin-app/app/admin/(app)/jobs/page.tsx` (rewrite)
- `packages/admin-app/app/admin/(app)/dashboard/page.tsx` (rewrite)

### Phase 9: Cleanup

- [x] Migrate admin Chatroom component from Restate to GraphQL
- [x] Remove unused imports across all migrated files
- [x] Verify zero `lib/restate` imports in admin pages and components
- [x] Run `yarn codegen` and `tsc --noEmit` to verify clean compilation
- [x] Delete `packages/admin-app/lib/restate/client.ts` and `types.ts` — public pages moved to web-app
- [x] Delete `packages/admin-app/app/api/restate/[...path]/route.ts` — public pages moved to web-app
- [x] Remove `swr` dependency from admin-app — no more consumers

**Completed (public pages moved to web-app):**
- [x] Delete `packages/admin-app/lib/restate/client.ts` and `types.ts`
- [x] Delete `packages/admin-app/app/api/restate/[...path]/route.ts`
- [x] Remove `swr` dependency
- [x] Delete `packages/admin-app/app/api/streams/[topic]/route.ts`
- [x] Delete `packages/admin-app/lib/hooks/usePublicChatStream.ts`

**Remaining (future):**
- Chat/SSE migration to GraphQL subscriptions

## Page Migration Pattern

Each page follows the same mechanical pattern:

```typescript
// BEFORE (SWR + Restate)
import { useRestate, callService, invalidateService } from "@/lib/restate/client";
import type { TagListResult } from "@/lib/restate/types";

const { data } = useRestate<TagListResult>("Tags", "list_tags", {});
await callService("Tags", "create_tag", { kind_id: kindId, value });
invalidateService("Tags");

// AFTER (urql + GraphQL)
import { useQuery, useMutation } from "urql";
import { TagsDocument, CreateTagDocument } from "@/gql/graphql";

const [{ data }] = useQuery({ query: TagsDocument });
const [, createTag] = useMutation(CreateTagDocument);
await createTag({ kindId, value }, { additionalTypenames: ["Tag"] });
```

## Acceptance Criteria

- [ ] admin-app GraphQL route imports from `@mntogether/shared` (not a stub)
- [ ] Zero imports from `@/lib/restate/client` in any admin-app page
- [ ] Zero imports from `@/lib/restate/types` in any admin-app page
- [ ] `lib/restate/` directory deleted from admin-app
- [ ] `/api/restate/[...path]` route deleted from admin-app
- [ ] `swr` removed from admin-app dependencies
- [ ] `yarn codegen` produces clean output in admin-app
- [ ] `yarn build` succeeds in admin-app
- [ ] All admin pages load and function identically to before
- [ ] All admin mutations work (approve, reject, CRUD, workflow triggers)
- [ ] requireAdmin enforced on all admin mutations

## Dependencies & Risks

- **Risk**: Large refactor touching every admin page (~130 operations across 16 flows). Mitigated by doing one domain at a time with testing between phases.
- **Risk**: RestateClient snakeToCamel transform may not handle all nested structures. Mitigated by existing tests on Posts domain.
- **Risk**: Cache invalidation behavior differs between SWR and urql. Mitigated by using `additionalTypenames` which triggers refetch of all queries that return those types.
- **Risk**: Long-running operations (AI/crawl) may timeout in GraphQL. Mitigated by fire-and-forget pattern returning workflow IDs for polling.
- **Risk**: Double snake_case/camelCase during incremental migration (SWR uses snake_case types, GraphQL returns camelCase). Mitigated by migrating complete pages at a time (no partial page migration).
- **Risk**: Auth regression — resolvers missing `requireAdmin` creates same vulnerability as security audit. Mitigated by explicit auth mapping table above and review per phase.
- **Risk**: Codegen misconfiguration — existing config points to `.graphql` files that don't exist. Mitigated by Phase 1 extracting SDL to `.graphql` files before any domain work.
- **Dependency**: Chat/SSE stays on Restate proxy — not blocked by this refactor.

## References

- GraphQL BFF plan: `docs/plans/2026-02-13-feat-graphql-bff-layer-plan.md`
- Auth security audit: `docs/plans/2026-02-12-fix-admin-route-authorization-security-audit-plan.md`
- Existing shared schema: `packages/shared/graphql/schema.ts`
- Existing post resolvers: `packages/shared/graphql/resolvers/post.ts`
- Restate client (to be replaced): `packages/admin-app/lib/restate/client.ts`
- Restate types (to be replaced): `packages/admin-app/lib/restate/types.ts`
