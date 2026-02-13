---
title: "Separate Web Apps from Admin App"
type: refactor
date: 2026-02-13
---

# Separate Web Apps from Admin App

## Overview

Split the monolithic `packages/web` (3,600+ files) into three focused Next.js apps — `admin-app`, `web-app`, and `search-app` — each communicating exclusively through the existing GraphQL layer in `packages/shared`.

```
packages/admin-app  ─┐
packages/web-app    ─┼─→ packages/shared (GraphQL) ─→ Restate ─→ Server
packages/search-app ─┘
```

## Problem Statement

`packages/web` has grown to 3,600+ TS/TSX files mixing admin functionality (dashboard, jobs, orgs, posts, proposals, sources, tags, websites) with public-facing features (about, contact, posts, submit, chat). This makes navigation harder, increases cognitive load, and means changes to admin can accidentally affect the public site.

## Key Decisions

- **Restate proxy carried forward temporarily**: Only ~5% of the data surface has GraphQL resolvers (posts). Admin pages still use `useRestate()`/`callService()` directly. The Restate proxy (`/api/restate/[...path]`) and SSE proxy (`/api/streams/[topic]`) stay in admin-app. Web-app and search-app start GraphQL-only.
- **Admin routes stay `/admin/*` prefixed**: Avoids rewriting middleware, sidebar, login flow, and every internal link. Can be simplified later.
- **Auth server actions exempt from GraphQL-only**: OTP flow (`lib/auth/actions.ts`) continues calling Restate directly via server actions. No GraphQL auth mutations needed yet.
- **Full schema exposed in all apps**: Auth enforced at resolver level (already uses `requireAuth`/`requireAdmin`).
- **Dev ports**: admin-app: 3000, web-app: 3001, search-app: 3002.
- **Use `git mv` for rename**: Preserves git history.

## Implementation Phases

### Phase 1: Rename packages/web → packages/admin-app

Mechanical rename, no feature changes.

1. Remove stale `packages/admin-app` directory (leftover from previous attempt — contains only `node_modules`, `.yarn`, `.next`)
   - `sudo rm -rf packages/admin-app`
2. `git mv packages/web packages/admin-app`
3. Update `packages/admin-app/package.json`:
   - `"name": "web"` → `"name": "admin-app"`
4. Update root `package.json` workspaces:
   - `"packages/web"` → `"packages/admin-app"`
5. Run `yarn install` from root to rewire workspaces
6. Verify `yarn dev` still works in `packages/admin-app`

**Files changed:**
- `packages/admin-app/package.json`
- `package.json` (root)

### Phase 2: Create packages/web-app

Fresh Next.js app with GraphQL boilerplate.

1. Create directory structure:
   ```
   packages/web-app/
   ├── app/
   │   ├── api/graphql/route.ts
   │   ├── layout.tsx
   │   ├── page.tsx
   │   └── globals.css
   ├── lib/
   │   └── urql-provider.tsx
   ├── codegen.ts
   ├── next.config.ts
   ├── postcss.config.mjs
   ├── tsconfig.json
   └── package.json
   ```

2. `package.json` — same deps as admin-app minus `react-markdown`, `swr`. Add `@mntogether/shared` workspace dependency. Dev script: `next dev --port 3001`.

3. `app/api/graphql/route.ts` — identical pattern to admin-app: import `typeDefs, resolvers` from `@mntogether/shared`, create Yoga server.

4. `lib/urql-provider.tsx` — copy from admin-app, unchanged.

5. `codegen.ts` — point schema to `../shared/graphql/typeDefs/**/*.graphql`, documents to local `app/**/*.tsx`, `app/**/*.ts`.

6. `next.config.ts` — copy from admin-app (turbopack root, transpilePackages, serverExternalPackages, security headers).

7. `tsconfig.json` — copy from admin-app (paths: `@/*` and `@shared/*`).

8. `postcss.config.mjs` — copy from admin-app.

9. `app/globals.css` — minimal: `@import "tailwindcss";` plus base body styles. No admin-specific animations.

10. `app/layout.tsx` — minimal root layout with GraphQLProvider, Inter font. Title: "MN Together".

11. `app/page.tsx` — placeholder home page.

12. Add `"packages/web-app"` to root `package.json` workspaces.

13. Run `yarn install`, verify `yarn dev` starts on port 3001, verify `/api/graphql` responds.

**Files created:**
- `packages/web-app/package.json`
- `packages/web-app/app/api/graphql/route.ts`
- `packages/web-app/app/layout.tsx`
- `packages/web-app/app/page.tsx`
- `packages/web-app/app/globals.css`
- `packages/web-app/lib/urql-provider.tsx`
- `packages/web-app/codegen.ts`
- `packages/web-app/next.config.ts`
- `packages/web-app/postcss.config.mjs`
- `packages/web-app/tsconfig.json`

**Files changed:**
- `package.json` (root — add workspace)

### Phase 3: Create packages/search-app

Same pattern as Phase 2.

1. Same directory structure as web-app.
2. Dev script: `next dev --port 3002`.
3. Title: "MN Together Search".
4. Placeholder search page.
5. Add `"packages/search-app"` to root `package.json` workspaces.
6. Run `yarn install`, verify starts on port 3002.

**Files created:** Same structure as web-app, in `packages/search-app/`.

**Files changed:**
- `package.json` (root — add workspace)

### Phase 4: Migrate public routes (incremental, future)

Not part of this initial plan. Once the skeleton apps are running:

1. Copy public routes from `admin-app/app/(public)/*` → `web-app/app/`
2. Copy `admin-app/components/public/*` → `web-app/components/`
3. Rewrite Restate hooks to use GraphQL queries (requires adding resolvers to shared schema)
4. Delete migrated routes and dead code from admin-app
5. Remove `admin-app/lib/restate/` once no pages depend on it

## Acceptance Criteria

- [ ] `packages/admin-app` exists, renamed from `packages/web` with git history preserved
- [ ] `packages/web-app` exists with working GraphQL endpoint at `/api/graphql`
- [ ] `packages/search-app` exists with working GraphQL endpoint at `/api/graphql`
- [ ] Root `package.json` workspaces lists all four packages
- [ ] `yarn install` succeeds from root
- [ ] `admin-app` starts on port 3000 and works identically to the old `packages/web`
- [ ] `web-app` starts on port 3001 and renders placeholder page
- [ ] `search-app` starts on port 3002 and renders placeholder page
- [ ] All three apps can query GraphQL (e.g., `publicPosts` query returns data)

## Open Questions (Deferred)

- Deployment topology (domains, reverse proxy, Docker services)
- Cookie sharing strategy for auth across apps
- Public chat SSE proxy for unauthenticated users
- Timeline for Restate-to-GraphQL migration of remaining admin pages
- Whether to split the GraphQL schema per-app or keep it unified

## References

- Brainstorm: `docs/brainstorms/2026-02-13-separate-web-apps-brainstorm.md`
- GraphQL BFF plan: `docs/plans/2026-02-13-feat-graphql-bff-layer-plan.md`
- Current shared package: `packages/shared/graphql/`
- Current web package: `packages/web/` (to become `packages/admin-app/`)
