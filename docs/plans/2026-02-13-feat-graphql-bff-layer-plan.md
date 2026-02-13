---
title: "feat: Add GraphQL BFF Layer"
type: feat
date: 2026-02-13
---

# Add GraphQL BFF Layer

## Overview

Replace the current Restate proxy (`/api/restate/[...path]`) and stringly-typed SWR hooks with a GraphQL BFF layer in the Next.js app. GraphQL becomes the **complete API boundary** — the frontend has zero knowledge of Restate.

```
Browser → urql (typed hooks) → /api/graphql → GraphQL Yoga → Dataloaders → Restate → Rust
```

## Problem Statement / Motivation

The current data fetching architecture has four pain points that worsen as the codebase grows:

1. **Multiple round-trips.** Fetching `{ organization { posts } }` requires separate `Organizations/public_get` + `Posts/public_list` calls. The frontend orchestrates joins that the API layer should handle.
2. **Over-fetching.** Every call returns the full response object. `PostResult` has 25+ fields — a list card needs 5.
3. **Ad-hoc frontend patterns.** `useRestate("Posts", "list", {...})` is string-based with no autocomplete, no type safety, and manual type annotations. Every developer has to know the Restate service names and handler names.
4. **No structural contract.** Types are manually duplicated between Rust and TypeScript (`types.ts`, 566 lines). No schema enforces the boundary.

## Proposed Solution

A schema-first GraphQL layer using GraphQL Yoga, urql, and graphql-codegen. Dataloaders batch Restate calls. Auth flows through GraphQL context. The schema is the single source of truth for the API contract.

## Technical Approach

### Stack

| Component | Choice | Rationale |
|-----------|--------|-----------|
| GraphQL server | GraphQL Yoga 5.x | WHATWG Fetch API, first-class Next.js route handler support, maintained by The Guild |
| Client | urql 4.x + @urql/next 1.x | Lightweight, document cache, suspense support, React 19 compatible |
| Schema | Schema-first SDL (.graphql files) | Schema IS the documentation, clean separation |
| Codegen | @graphql-codegen/cli + client-preset | Generates TypedDocumentNode + typed hooks from SDL + operations |
| Batching | dataloader 2.x | Per-request instantiation, deduplication and batching of Restate calls |
| SSE (subscriptions) | Deferred (Phase 5) | Existing SSE proxy stays until query/mutation layer is proven |

### Key Architectural Decisions

**1. Auth stays as server actions for OTP, enters GraphQL for everything else.**

The `verifyOtp` server action sets an httpOnly cookie via `cookies().set()`. Moving this into a GraphQL mutation would require custom response header manipulation in Yoga — awkward and fragile. OTP flows (send_otp, verify_otp, logout) remain server actions. All other authenticated operations go through GraphQL context.

GraphQL context reads the `auth_token` cookie, creates a `RestateClient` with the token baked in, and passes it to all dataloaders/resolvers:

```typescript
// graphql/context.ts
export async function createContext({ request }: YogaInitialContext): Promise<GraphQLContext> {
  const cookieHeader = request.headers.get("cookie") || "";
  const token = parseCookie(cookieHeader, "auth_token");

  const restateClient = new RestateClient({ token });

  return {
    user: token ? restateClient.decodeTokenClaims(token) : null,
    restate: restateClient,
    loaders: createLoaders(restateClient),
  };
}
```

The `user` object contains `{ memberId, phoneNumber, isAdmin }` decoded from the JWT. Actual JWT validation still happens in the Rust backend — the GraphQL layer does the same pass-through the current proxy does, but also extracts claims for resolver-level auth checks.

**2. Authorization is resolver-level with context helpers.**

```typescript
// graphql/auth.ts
export function requireAuth(ctx: GraphQLContext): AuthUser {
  if (!ctx.user) throw new GraphQLError("Authentication required", { extensions: { code: "UNAUTHENTICATED" } });
  return ctx.user;
}

export function requireAdmin(ctx: GraphQLContext): AuthUser {
  const user = requireAuth(ctx);
  if (!user.isAdmin) throw new GraphQLError("Admin access required", { extensions: { code: "FORBIDDEN" } });
  return user;
}
```

Public fields have no auth check. Admin fields call `requireAuth()` or `requireAdmin()`. The `INTERNAL_ONLY_PATHS` (workflow triggers like `CrawlWebsite`, `ExtractPostsFromUrl`) are simply **not exposed in the schema** — they don't exist in GraphQL.

**3. GraphQL schema uses camelCase. RestateClient transforms automatically.**

Restate returns snake_case. GraphQL convention is camelCase. The `RestateClient` has a generic response transformer that converts keys:

```typescript
// graphql/restate-client.ts
export class RestateClient {
  async callService<T>(service: string, handler: string, body?: unknown): Promise<T> {
    const raw = await this.fetch(`${service}/${handler}`, body);
    return snakeToCamel(raw) as T;
  }

  async callObject<T>(object: string, key: string, handler: string, body?: unknown): Promise<T> {
    const raw = await this.fetch(`${object}/${key}/${handler}`, body);
    return snakeToCamel(raw) as T;
  }
}
```

This means the SDL schema uses camelCase (`createdAt`, `postType`, `organizationId`) and resolver return values already match.

**4. urql document cache + refetchQueries after mutations.**

The simplest cache strategy that maps to the current SWR invalidation pattern. After mutations, specified queries are refetched:

```typescript
const [, approvePost] = useMutation(ApprovePostMutation);

await approvePost({ postId }, {
  // Refetch these queries after mutation succeeds
  additionalTypenames: ["Post", "PostConnection"],
});
```

No normalized cache (graphcache) initially. We can adopt it later if needed.

**5. Fire-and-forget calls become GraphQL mutations returning Boolean.**

`trackView`, `trackClick` become mutations to maintain the "complete API boundary" principle:

```graphql
type Mutation {
  trackPostView(postId: ID!): Boolean
  trackPostClick(postId: ID!): Boolean
}
```

Resolvers fire the Restate call and return `true`. Failures return `false` silently — no error thrown.

**6. Workflow mutations return a `Job` type.**

Long-running workflows (crawl, extract, deduplicate, etc.) return a job ID immediately:

```graphql
type Job {
  id: ID!
  status: String!
  message: String
}

type Mutation {
  crawlWebsite(websiteId: ID!): Job!
  extractPostsFromUrl(url: String!): Job!
}

type Query {
  job(id: ID!): Job
  jobs(limit: Int, offset: Int): [Job!]!
}
```

**7. SSE/Subscriptions deferred to Phase 5.**

The existing `/api/streams/[topic]` proxy stays. Chat streaming, post update SSE — all continue through the current proxy during the migration. GraphQL subscriptions via Yoga's built-in SSE support will wrap the Rust SSE server in a future phase.

**8. Pagination: offset-based, matching current patterns.**

```graphql
type PostConnection {
  posts: [Post!]!
  totalCount: Int!
  hasNextPage: Boolean!
  hasPreviousPage: Boolean!
}

type Query {
  posts(status: String, limit: Int, offset: Int): PostConnection!
}
```

Relay-style cursors can be adopted later if needed.

**9. All pages stay as client components using urql hooks.**

Server-side GraphQL queries (via `@urql/next` RSC support) are deferred. The initial migration swaps SWR hooks for urql hooks in existing "use client" components.

### File Structure

```
packages/web/
  app/api/graphql/
    route.ts                          # GraphQL Yoga route handler
  graphql/
    schema.ts                         # Schema assembly (loadFilesSync + merge)
    context.ts                        # Context factory (auth + loaders)
    restate-client.ts                 # RestateClient class (wraps all Restate HTTP calls)
    auth.ts                           # requireAuth, requireAdmin helpers
    util.ts                           # snakeToCamel transformer
    typeDefs/
      base.graphql                    # Root Query, Mutation, scalar types
      post.graphql                    # Post, PostConnection, post queries/mutations
      organization.graphql            # Organization, org queries
      website.graphql                 # Website, website queries/mutations
      member.graphql                  # Member queries
      tag.graphql                     # Tag, TagKind queries/mutations
      note.graphql                    # Note queries/mutations
      source.graphql                  # Source queries/mutations
      sync.graphql                    # SyncBatch, SyncProposal queries/mutations
      job.graphql                     # Job queries
      chat.graphql                    # Chatroom, ChatMessage queries/mutations
      provider.graphql                # Provider queries
      search.graphql                  # SearchQuery queries
      heat-map.graphql                # HeatMap queries
    resolvers/
      index.ts                        # mergeResolvers
      post.ts                         # Post resolvers
      organization.ts                 # Organization resolvers
      website.ts                      # Website resolvers
      member.ts                       # Member resolvers
      tag.ts                          # Tag resolvers
      note.ts                         # Note resolvers
      source.ts                       # Source resolvers
      sync.ts                         # Sync resolvers
      job.ts                          # Job resolvers
      chat.ts                         # Chat resolvers
      provider.ts                     # Provider resolvers
      search.ts                       # Search resolvers
      heat-map.ts                     # HeatMap resolvers
    dataloaders/
      index.ts                        # createLoaders factory
      post.ts                         # PostLoader, PostsByOrgLoader
      organization.ts                 # OrganizationLoader
      member.ts                       # MemberLoader
      tag.ts                          # TagLoader
      website.ts                      # WebsiteLoader
      source.ts                       # SourceLoader
  gql/                                # Generated by graphql-codegen (gitignored)
    graphql.ts
    gql.ts
    index.ts
  lib/
    urql-provider.tsx                 # urql client + UrqlProvider
    urql-rsc.ts                       # Server component client (for future use)
  codegen.ts                          # graphql-codegen configuration
```

### Schema Design (Core Types)

```graphql
# graphql/typeDefs/base.graphql
type Query {
  # Public
  publicPosts(postType: String, category: String, limit: Int, offset: Int, zipCode: String, radiusMiles: Float): PublicPostConnection!
  publicFilters: PublicFilters!
  publicOrganizations: [Organization!]!
  post(id: ID!): Post
  organization(id: ID!): Organization

  # Admin (auth required)
  posts(status: String, search: String, limit: Int, offset: Int): PostConnection!
  websites(limit: Int, offset: Int): WebsiteConnection!
  sources(limit: Int, offset: Int): SourceConnection!
  organizations: [Organization!]!
  members: [Member!]!
  tags(kindSlug: String): [Tag!]!
  tagKinds: [TagKind!]!
  syncBatches(limit: Int, offset: Int): [SyncBatch!]!
  syncProposals(batchId: ID!): [SyncProposal!]!
  jobs(limit: Int, offset: Int): [Job!]!
  job(id: ID!): Job
  notes(entityId: ID, entityType: String): [Note!]!
  searchQueries: [SearchQuery!]!
  chatrooms: [Chatroom!]!
  heatMapData(zipCode: String, radiusMiles: Float): HeatMapData
}

type Mutation {
  # Public
  submitPost(input: SubmitPostInput!): Post!
  submitResourceLink(url: String!): Job!
  addComment(postId: ID!, content: String!, parentMessageId: ID): Comment!
  trackPostView(postId: ID!): Boolean
  trackPostClick(postId: ID!): Boolean
  createChat(withAgent: String): Chatroom!
  sendMessage(chatroomId: ID!, content: String!): ChatMessage!

  # Admin (auth required)
  approvePost(postId: ID!): Post!
  rejectPost(postId: ID!, reason: String): Post!
  archivePost(postId: ID!): Post!
  reactivatePost(postId: ID!): Post!
  deletePost(postId: ID!): Boolean!
  regeneratePost(postId: ID!): Post!
  regeneratePostTags(postId: ID!): Post!
  updatePostTags(postId: ID!, addTagIds: [ID!], removeTagIds: [ID!]): Post!
  submitWebsite(url: String!): Website!
  approveWebsite(websiteId: ID!): Website!
  crawlWebsite(websiteId: ID!): Job!
  crawlSource(sourceId: ID!): Job!
  approveProposal(proposalId: ID!): SyncProposal!
  rejectProposal(proposalId: ID!, reason: String): SyncProposal!
  createNote(input: CreateNoteInput!): Note!
}

# graphql/typeDefs/post.graphql
type Post {
  id: ID!
  title: String!
  description: String!
  descriptionMarkdown: String
  summary: String
  status: String!
  postType: String
  category: String
  capacityStatus: String
  urgency: String
  location: String
  sourceUrl: String
  submissionType: String
  createdAt: String!
  updatedAt: String!
  publishedAt: String
  organizationId: ID
  organizationName: String
  distanceMiles: Float
  relevanceScore: Float
  relevanceBreakdown: String
  hasUrgentNotes: Boolean

  # Nested resolvers (use dataloaders)
  tags: [Tag!]!
  schedules: [PostSchedule!]!
  contacts: [PostContact!]!
  submittedBy: SubmittedByInfo
  urgentNotes: [UrgentNote!]!
  comments: [Comment!]!
  organization: Organization
  sourcePages: [SourcePage!]!
  notes: [Note!]!
  entityProposals: [SyncProposal!]!
}

type PostConnection {
  posts: [Post!]!
  totalCount: Int!
  hasNextPage: Boolean!
  hasPreviousPage: Boolean!
}

# graphql/typeDefs/organization.graphql
type Organization {
  id: ID!
  name: String!
  description: String
  status: String!
  websiteCount: Int!
  socialProfileCount: Int!
  snapshotCount: Int!
  createdAt: String!
  updatedAt: String!

  # Nested resolvers
  posts: [Post!]!              # <-- This is the key join: { organization { posts } }
  websites: [Website!]!
  socialProfiles: [SocialProfile!]!
}
```

### Dataloader Design

Dataloaders are created per-request in the context factory. Each dataloader wraps a Restate call via `RestateClient`:

```typescript
// graphql/dataloaders/index.ts
export interface DataLoaders {
  postById: DataLoader<string, Post>;
  postsByOrgId: DataLoader<string, Post[]>;
  organizationById: DataLoader<string, Organization>;
  tagsByPostId: DataLoader<string, Tag[]>;
  commentsByPostId: DataLoader<string, Comment[]>;
  notesByEntityId: DataLoader<string, Note[]>;
  websitesByOrgId: DataLoader<string, Website[]>;
  memberById: DataLoader<string, Member>;
}

export function createLoaders(restate: RestateClient): DataLoaders {
  return {
    postById: new DataLoader(async (ids) => {
      // Batch: call Post/{id}/get for each ID
      const results = await Promise.all(
        ids.map((id) => restate.callObject<Post>("Post", id, "get", { show_private: false }))
      );
      return results;
    }),

    postsByOrgId: new DataLoader(async (orgIds) => {
      // For each org, fetch its posts
      const results = await Promise.all(
        orgIds.map((orgId) =>
          restate.callService<PostList>("Posts", "list", { organization_id: orgId })
            .then((r) => r.posts)
        )
      );
      return results;
    }),

    organizationById: new DataLoader(async (ids) => {
      const results = await Promise.all(
        ids.map((id) => restate.callService<Organization>("Organizations", "public_get", { id }))
      );
      return results;
    }),

    // ... more loaders
  };
}
```

**Key insight:** Restate doesn't have batch endpoints, so dataloaders primarily provide **per-request deduplication** (if Post #123 is referenced 3 times in one query, it's fetched once) and **a clean resolver pattern** (resolvers don't know about Restate, they load from dataloaders).

### RestateClient

```typescript
// graphql/restate-client.ts
const RESTATE_INGRESS_URL = process.env.RESTATE_INGRESS_URL || "http://localhost:8180";
const RESTATE_AUTH_TOKEN = process.env.RESTATE_AUTH_TOKEN || "";

export class RestateClient {
  private token: string | null;

  constructor({ token }: { token: string | null }) {
    this.token = token;
  }

  async callService<T>(service: string, handler: string, body?: unknown): Promise<T> {
    return this.call<T>(`${service}/${handler}`, body);
  }

  async callObject<T>(object: string, key: string, handler: string, body?: unknown): Promise<T> {
    return this.call<T>(`${object}/${key}/${handler}`, body);
  }

  private async call<T>(path: string, body?: unknown): Promise<T> {
    const headers: HeadersInit = { "Content-Type": "application/json" };
    if (RESTATE_AUTH_TOKEN) headers["Authorization"] = `Bearer ${RESTATE_AUTH_TOKEN}`;
    if (this.token) headers["X-User-Token"] = this.token;

    const response = await fetch(`${RESTATE_INGRESS_URL}/${path}`, {
      method: "POST",
      headers,
      body: JSON.stringify(body ?? {}),
    });

    if (!response.ok) {
      const text = await response.text();
      let message: string;
      try { message = JSON.parse(text).message || text; } catch { message = text; }
      throw new GraphQLError(message, {
        extensions: {
          code: response.status === 401 ? "UNAUTHENTICATED"
              : response.status === 403 ? "FORBIDDEN"
              : response.status === 404 ? "NOT_FOUND"
              : "INTERNAL_SERVER_ERROR",
        },
      });
    }

    const raw = await response.json();
    return snakeToCamel(raw) as T;
  }
}
```

### urql Provider

```typescript
// lib/urql-provider.tsx
"use client";

import { useMemo, type ReactNode } from "react";
import { UrqlProvider, ssrExchange, cacheExchange, fetchExchange, createClient } from "@urql/next";

export default function GraphQLProvider({ children }: { children: ReactNode }) {
  const [client, ssr] = useMemo(() => {
    const ssr = ssrExchange({ isClient: typeof window !== "undefined" });
    const client = createClient({
      url: "/api/graphql",
      exchanges: [cacheExchange, ssr, fetchExchange],
      suspense: true,
      fetchOptions: { credentials: "same-origin" }, // sends httpOnly cookie
    });
    return [client, ssr] as const;
  }, []);

  return (
    <UrqlProvider client={client} ssr={ssr}>
      {children}
    </UrqlProvider>
  );
}
```

### Codegen Configuration

```typescript
// codegen.ts
import type { CodegenConfig } from "@graphql-codegen/cli";

const config: CodegenConfig = {
  schema: "./graphql/typeDefs/**/*.graphql",
  documents: [
    "./app/**/*.tsx",
    "./app/**/*.ts",
    "./components/**/*.tsx",
    "./components/**/*.ts",
    "./lib/**/*.ts",
    "!./gql/**/*",
  ],
  ignoreNoDocuments: true,
  generates: {
    "./gql/": {
      preset: "client",
      presetConfig: { fragmentMasking: false },
      config: {
        scalars: { DateTime: "string", UUID: "string" },
      },
    },
  },
};

export default config;
```

---

## Implementation Phases

### Phase 1: Foundation (scaffold + first query)

Set up the GraphQL infrastructure and prove the pattern with one query.

**Tasks:**
- [ ] Install dependencies: `graphql`, `graphql-yoga`, `@graphql-tools/load-files`, `@graphql-tools/merge`, `dataloader`, `urql`, `@urql/next`, `@urql/core`, `@graphql-codegen/cli`, `@graphql-codegen/client-preset`
- [ ] Create `packages/web/graphql/restate-client.ts` — `RestateClient` class
- [ ] Create `packages/web/graphql/util.ts` — `snakeToCamel` deep transformer
- [ ] Create `packages/web/graphql/auth.ts` — `requireAuth`, `requireAdmin` helpers
- [ ] Create `packages/web/graphql/context.ts` — context factory
- [ ] Create `packages/web/graphql/typeDefs/base.graphql` — root Query/Mutation stubs
- [ ] Create `packages/web/graphql/typeDefs/post.graphql` — Post types and public queries
- [ ] Create `packages/web/graphql/dataloaders/index.ts` — loader factory
- [ ] Create `packages/web/graphql/dataloaders/post.ts` — PostLoader
- [ ] Create `packages/web/graphql/resolvers/index.ts` — mergeResolvers
- [ ] Create `packages/web/graphql/resolvers/post.ts` — publicPosts, post(id) resolvers
- [ ] Create `packages/web/graphql/schema.ts` — schema assembly
- [ ] Create `packages/web/app/api/graphql/route.ts` — Yoga route handler
- [ ] Create `packages/web/codegen.ts` — graphql-codegen config
- [ ] Create `packages/web/lib/urql-provider.tsx` — urql provider component
- [ ] Add `gql/` to `.gitignore`
- [ ] Add `codegen` and `codegen:watch` scripts to `package.json`
- [ ] Wrap root layout with `<GraphQLProvider>`
- [ ] Write one GraphQL operation in a test component to verify the full pipeline

**Verification:** Run `yarn codegen`, see generated types. Visit GraphiQL at `/api/graphql`. Run `publicPosts` query, see data.

### Phase 2: Public Read Queries

Migrate all public-facing read operations to GraphQL.

**Tasks:**
- [ ] Add SDL types: `organization.graphql`, `tag.graphql`
- [ ] Add resolvers: `organization.ts`, `tag.ts`
- [ ] Add dataloaders: `organization.ts`, `tag.ts`
- [ ] Add nested resolvers: `Organization.posts`, `Post.organization`, `Post.tags`, `Post.schedules`, `Post.contacts`
- [ ] Write GraphQL operations for public pages:
  - `PublicPostsQuery` (PostFeed)
  - `PublicFiltersQuery` (PostFeed)
  - `PostDetailQuery` (public post detail — post + comments + organization)
  - `PublicOrganizationsQuery` (organizations list)
  - `OrganizationDetailQuery` (org detail with posts)
- [ ] Migrate `packages/web/components/public/PostFeed.tsx` to urql hooks
- [ ] Migrate `packages/web/app/(public)/posts/[id]/page.tsx` to urql hooks
- [ ] Migrate `packages/web/app/(public)/organizations/page.tsx` to urql hooks
- [ ] Migrate `packages/web/app/(public)/organizations/[id]/page.tsx` to urql hooks
- [ ] Migrate `packages/web/components/public/CommentsSection.tsx` to urql hooks

**Verification:** All public pages render correctly with data from GraphQL. No calls to `/api/restate/` from public pages.

### Phase 3: Public Mutations + Admin Read Queries

**Tasks:**
- [ ] Add SDL types: `website.graphql`, `source.graphql`, `member.graphql`, `note.graphql`, `sync.graphql`, `job.graphql`, `chat.graphql`, `provider.graphql`, `search.graphql`, `heat-map.graphql`
- [ ] Add all remaining resolvers and dataloaders
- [ ] Public mutations:
  - `submitPost`, `submitResourceLink` (SubmitSheet)
  - `addComment` (CommentsSection)
  - `trackPostView`, `trackPostClick` (fire-and-forget)
  - `createChat`, `sendMessage` (ChatSheet)
- [ ] Admin read queries:
  - `PostsQuery` with filters (admin posts list)
  - `PostDetailQuery` admin variant (with `showPrivate: true`, notes, proposals)
  - `WebsitesQuery`, `WebsiteDetailQuery`
  - `SourcesQuery`, `SourceDetailQuery`
  - `OrganizationsQuery` (admin variant)
  - `TagsQuery`, `TagKindsQuery`
  - `SyncBatchesQuery`, `SyncProposalsQuery`
  - `JobsQuery`, `JobDetailQuery`
  - `MembersQuery`
  - `ChatroomsQuery`, `ChatMessagesQuery`
  - `SearchQueriesQuery`
  - `HeatMapQuery`
- [ ] Migrate admin list pages to urql hooks
- [ ] Migrate admin detail pages to urql hooks
- [ ] Migrate public mutation components (SubmitSheet, CommentsSection)

**Verification:** All admin pages and public mutations work through GraphQL. `/api/restate/` calls only remain for admin mutations and SSE.

### Phase 4: Admin Mutations

**Tasks:**
- [ ] Post mutations: `approvePost`, `rejectPost`, `archivePost`, `reactivatePost`, `deletePost`, `regeneratePost`, `regeneratePostTags`, `updatePostTags`
- [ ] Website mutations: `submitWebsite`, `approveWebsite`, `crawlWebsite`
- [ ] Source mutations: `crawlSource`
- [ ] Sync mutations: `approveProposal`, `rejectProposal`
- [ ] Note mutations: `createNote`
- [ ] Migrate admin action components (approve/reject buttons, tag modals, etc.)
- [ ] Implement cache invalidation via `additionalTypenames` on mutations
- [ ] Test all admin workflows end-to-end

**Verification:** All admin actions work through GraphQL. Zero calls to `/api/restate/`.

### Phase 5: SSE Subscriptions + Cleanup

**Tasks:**
- [ ] Design GraphQL subscription schema for chat and post updates
- [ ] Implement Yoga PubSub that subscribes to the Rust SSE server upstream
- [ ] Add `subscriptionExchange` + `graphql-sse` to urql provider
- [ ] Migrate `useChatStream` and `usePublicChatStream` to GraphQL subscriptions
- [ ] Delete `packages/web/app/api/restate/[...path]/route.ts` (proxy)
- [ ] Delete `packages/web/app/api/streams/[topic]/route.ts` (SSE proxy)
- [ ] Delete `packages/web/lib/restate/client.ts` (SWR hooks)
- [ ] Delete `packages/web/lib/restate/server.ts` (server-side client, except auth)
- [ ] Delete `packages/web/lib/restate/types.ts` (manual types)
- [ ] Remove `swr` from `package.json`
- [ ] Audit: confirm zero imports from `lib/restate/`

**Verification:** The entire frontend communicates exclusively through `/api/graphql`. No Restate knowledge in any frontend file.

---

## Alternative Approaches Considered

**tRPC:** Full end-to-end type safety, simpler than GraphQL. Rejected because it doesn't support field selection or nested relational queries (`{ organization { posts } }`). Would feel like a typed version of the current pattern rather than an improvement.

**Purpose-built BFF routes:** Keep SWR, create `/api/views/post-detail` etc. Rejected because it doesn't scale — a bespoke route for every page, no schema contract, still maintaining types manually.

**Apollo Server + Apollo Client:** More mature ecosystem but heavier. Apollo Client's normalized cache adds complexity we don't need initially. Yoga + urql is lighter and has better Next.js integration.

## Acceptance Criteria

### Functional Requirements

- [ ] All public pages fetch data through GraphQL queries
- [ ] All admin pages fetch data through GraphQL queries
- [ ] All mutations (public and admin) go through GraphQL
- [ ] Nested queries work: `{ organization { posts { tags } } }` in single request
- [ ] Field selection works: requesting only `{ posts { id title } }` returns only those fields
- [ ] Auth flows work: login, admin operations, protected queries
- [ ] Fire-and-forget analytics work: view/click tracking
- [ ] Workflow mutations return job objects
- [ ] GraphiQL available in development at `/api/graphql`

### Non-Functional Requirements

- [ ] Zero Restate knowledge in frontend code (no imports from `lib/restate/`)
- [ ] All GraphQL operations are fully typed via codegen
- [ ] Dataloaders prevent N+1 Restate calls within a request
- [ ] Error responses use structured GraphQL error format with codes
- [ ] GraphQL introspection disabled in production

### Quality Gates

- [ ] `yarn codegen` produces clean output with no errors
- [ ] `yarn build` succeeds with generated types
- [ ] All existing functionality works identically after migration

## Dependencies & Prerequisites

- GraphQL Yoga 5.x compatible with Next.js 16 route handlers
- urql @urql/next compatible with React 19
- graphql-codegen client-preset compatible with urql 4.x
- No changes needed to the Rust backend — GraphQL wraps existing Restate endpoints

## Risk Analysis & Mitigation

| Risk | Impact | Mitigation |
|------|--------|------------|
| urql + React 19 + Turbopack incompatibility | Dev workflow broken | Use Webpack bundler, pin urql versions, test early in Phase 1 |
| Dataloader doesn't batch effectively (no batch Restate endpoints) | Performance same as today | Dataloaders still deduplicate; add batch endpoints to Rust later if needed |
| Migration leaves codebase in dual-API state for too long | Confusion, bugs | Time-box each phase. Both systems work during migration — no half-states |
| camelCase transform breaks on edge cases (nested arrays, null values) | Runtime errors | Write comprehensive tests for snakeToCamel with all Restate response shapes |
| graphql-codegen watch mode slow | Dev experience degrades | Only generate on save, not continuous. Consider `--watch` with debounce |

## Open Questions (Deferred)

- **Server components:** Should public pages move to RSC with server-side GraphQL queries? Deferred — get the client-side migration right first.
- **Normalized cache:** Should we adopt `@urql/exchange-graphcache` for smarter cache updates after mutations? Deferred — document cache is sufficient initially.
- **Relay pagination:** Should we adopt cursor-based pagination? Deferred — offset-based matches current Restate patterns.
- **Subscription architecture:** How exactly does Yoga PubSub connect to the Rust SSE server? Deferred to Phase 5.

## References

### Internal
- Current Restate proxy: `packages/web/app/api/restate/[...path]/route.ts`
- Current SWR client: `packages/web/lib/restate/client.ts`
- Current server client: `packages/web/lib/restate/server.ts`
- Current types: `packages/web/lib/restate/types.ts` (566 lines, ~30 types)
- Restate service registrations: `packages/server/src/bin/server.rs` (35+ endpoints)
- SSE proxy: `packages/web/app/api/streams/[topic]/route.ts`
- Auth server actions: `packages/web/lib/auth/actions.ts`

### External
- [GraphQL Yoga + Next.js Integration](https://the-guild.dev/graphql/yoga-server/docs/integrations/integration-with-nextjs)
- [urql + Next.js App Router (@urql/next)](https://github.com/urql-graphql/urql/tree/main/packages/next-urql)
- [graphql-codegen Client Preset](https://the-guild.dev/graphql/codegen/plugins/presets/preset-client)
- [DataLoader Best Practices](https://github.com/graphql/dataloader)
- [GraphQL Yoga Subscriptions (SSE)](https://the-guild.dev/graphql/yoga-server/docs/features/subscriptions)
