---
date: 2026-02-13
topic: separate-web-apps
---

# Separate Web Apps from Admin App

## What We're Building

Split the monolithic `packages/web` into separate, focused Next.js apps: `admin-app`, `web-app`, and `search-app`. Each app is standalone and communicates with the backend exclusively through GraphQL via `packages/shared`.

## Architecture

```
packages/admin-app  ─┐
packages/web-app    ─┼─→ packages/shared (GraphQL) ─→ Restate ─→ Server
packages/search-app ─┘
```

Web apps only speak GraphQL via urql. They know nothing about Restate.

## Four Packages

| Package | Purpose |
|---------|---------|
| `packages/admin-app` | Renamed from `packages/web`. Admin panel (dashboard, jobs, organizations, posts, proposals, search-queries, sources, tags, websites). |
| `packages/web-app` | New. Public-facing site (about, contact, organizations, posts, submit, chat, home). |
| `packages/search-app` | New. Search engine. |
| `packages/shared` | Already exists. GraphQL schema, resolvers, dataloaders, restate-client. |

## Each Web App Bootstraps With

- `next.config.ts`
- `app/api/graphql/route.ts` — imports schema from `@mntogether/shared`
- `codegen.ts` — points to `../shared/graphql/typeDefs/**/*.graphql`
- `lib/urql-provider.tsx` — urql client configured to `/api/graphql`
- `gql/` — generated typed hooks (output of codegen)
- Own components, styles, pages

## Migration Plan

1. Rename `packages/web` → `packages/admin-app`
2. Create fresh `packages/web-app` and `packages/search-app`
3. Each gets the same GraphQL boilerplate (api/graphql route, urql-provider, codegen)
4. Copy public routes from `admin-app` → `web-app` over time
5. Delete dead code from `admin-app` (public routes, old `lib/restate/` direct calls)

## Why This Approach

- **Code isolation**: Admin, public, and search are separate concerns with different users, different auth needs, and different deployment profiles
- **Start fresh, copy incrementally**: No big-bang migration. New apps start empty, functionality moves over piece by piece
- **Shared GraphQL layer already exists**: `packages/shared` is the right boundary — all apps talk GraphQL, none know about Restate
- **Dead code cleanup**: Old `lib/restate/` direct calls in `packages/web` can be removed as routes migrate to GraphQL

## Key Decisions

- **No shared UI components for now**: Each app owns its own components. Extract to shared only if duplication becomes a problem.
- **GraphQL is the only API layer**: Web apps never call Restate directly.
- **Copy, don't extract**: Start fresh apps and copy functionality over rather than trying to surgically split the existing codebase.

## Open Questions

- Domains / deployment topology for each app
- Styling / design system for web-app and search-app
- Timeline for cleaning up old `lib/restate/` code in admin-app

## Next Steps

→ `/workflows:plan` for implementation details
