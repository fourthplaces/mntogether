# Postmortem: Docker Anonymous Volumes Cause Stale node_modules

**Date**: 2026-02-28
**Duration**: ~2 hours of investigation across two sessions
**Impact**: Admin app failed to start — `Can't resolve 'clsx'`, `Can't resolve 'class-variance-authority'` after adding new npm dependencies
**Resolution**: Structural fix — entrypoint script that auto-detects stale deps via yarn.lock hash comparison
**Root cause**: Confirmed — Docker anonymous volumes persist old node_modules across image rebuilds

---

## Timeline

| Time | Event |
|------|-------|
| ~T+0 | shadcn/ui migration added 6 new npm packages (`clsx`, `class-variance-authority`, `tailwind-merge`, `tw-animate-css`, `lucide-react`, `radix-ui`) via `yarn add` |
| ~T+5m | `docker compose up -d --build admin-app` completed successfully (image rebuilt with fresh `yarn install`) |
| ~T+10m | Admin app fails at runtime: `Module not found: Can't resolve 'class-variance-authority'` |
| ~T+15m | Confirmed packages exist in host's root `node_modules/` (hoisted by Yarn workspaces). Local `yarn dev` would work fine — but the app runs in Docker |
| ~T+20m | First wrong fix: added `outputFileTracingRoot` to `next.config.ts`. No effect — this is a monorepo tracing hint, not a module resolution fix |
| ~T+30m | Second wrong fix: created manual symlinks in local `packages/admin-app/node_modules/` pointing to root `node_modules/`. No effect inside Docker — the anonymous volume shadows local files |
| ~T+40m | Key revelation: user runs `dev.sh`, which uses Docker Compose for everything. Not local `yarn dev` |
| ~T+50m | Identified the pattern in `docker-compose.yml`: anonymous volumes `/app/node_modules` and `/app/packages/admin-app/node_modules` |
| ~T+55m | Tested hypothesis: `docker compose down admin-app -v && docker compose build --no-cache admin-app && docker compose up -d admin-app` |
| ~T+60m | ✅ Admin app starts successfully. All new packages resolve. Confirms anonymous volume was serving stale deps |
| ~T+70m | User asks: "is there a root issue to fix here? i dont think this is the first time we've had this genre of bug" |
| ~T+80m | Designed and implemented entrypoint-based fix with yarn.lock hash comparison |

## What Happened

The admin app's `Dockerfile.dev` runs `yarn install` at **image build time**, which correctly installs all dependencies into `/app/node_modules`. However, `docker-compose.yml` declares `/app/node_modules` as an **anonymous volume** — a Docker feature that prevents the host bind mount (`./packages/admin-app:/app/packages/admin-app`) from overwriting the container's `node_modules` with platform-incompatible host binaries.

The trap: anonymous volumes are created **once** and **reused forever**. When `docker compose up -d --build` runs, Docker rebuilds the image (with fresh deps), but **mounts the old anonymous volume over the image's `/app/node_modules`**. The new packages exist in the image layer but never reach the running container.

This is not specific to shadcn/ui. **Any `yarn add` of a new package** triggers the same failure. The only workaround was `docker compose down -v` (which destroys all anonymous volumes), but this is non-obvious and easy to forget.

## The Anonymous Volume Pattern

```
Image build time:         yarn install → /app/node_modules (in image layer)
                                          ├── clsx ✅
                                          ├── class-variance-authority ✅
                                          └── ... (all packages)

Container start time:     anonymous volume mounted over /app/node_modules
                                          ├── (OLD packages from first-ever container start)
                                          └── clsx ❌ (doesn't exist in old volume)
```

Both Next.js apps (`admin-app` and `web-app`) use this pattern:

```yaml
volumes:
  - ./packages/admin-app:/app/packages/admin-app  # host source code
  - /app/node_modules                              # anonymous volume (stale trap)
  - /app/packages/admin-app/node_modules           # anonymous volume (stale trap)
  - /app/packages/admin-app/.next                  # anonymous volume (cache)
```

## Investigation Results

### What we confirmed

| Hypothesis | Test | Result |
|-----------|------|--------|
| Packages missing from container | `docker compose exec admin-app ls /app/node_modules/clsx` | ✅ `No such file or directory` — confirmed stale volume |
| Packages present in rebuilt image | `docker compose run --rm admin-app ls /app/node_modules/clsx` (uses image, not volume) | ✅ Directory exists — image has correct deps |
| Volume removal fixes it | `docker compose down admin-app -v && docker compose up -d admin-app` | ✅ All packages resolve |
| `outputFileTracingRoot` irrelevant | Added config, restarted — same error | ❌ No effect (this is a production tracing hint) |
| Local symlinks irrelevant | Created symlinks in host's `packages/admin-app/node_modules/` | ❌ No effect (anonymous volume shadows them inside Docker) |
| Same bug class as Turbopack postmortem | Previous CPU loop fix also required `docker compose down` to clear anonymous volumes | ✅ Same root cause family — stale anonymous volumes |

### Connection to Turbopack CPU Loop

The [Turbopack postmortem](./TURBOPACK_CPU_LOOP_POSTMORTEM.md) from 2026-02-26 noted:

> **What we could not determine:** Whether it was in-memory state or on-disk cache. Our fix destroyed both simultaneously.

In hindsight, that incident may also have involved stale anonymous volume state. The `.next` anonymous volume preserves Turbopack's disk cache across container restarts, meaning corrupted compilation state would persist until `docker compose down -v`. The entrypoint fix addresses the `node_modules` volumes; the `.next` volume remains anonymous but is less likely to cause hard failures (Turbopack invalidates its own cache on file changes).

## Mistakes During Investigation

1. **Assumed local dev environment.** Spent 20+ minutes debugging module resolution as if the app ran locally (`yarn dev`), not in Docker. The `CLAUDE.md` dev setup section says `cd packages/admin-app && yarn dev`, which is misleading — the actual workflow is `./dev.sh` → Docker. Lesson: **always confirm the actual runtime environment** before debugging module resolution.

2. **Applied irrelevant fixes.** `outputFileTracingRoot` and manual symlinks were both reasonable for a local Yarn workspaces issue, but completely ineffective inside Docker. Lesson: **diagnose first, fix second.** Should have run `docker compose exec admin-app ls /app/node_modules/clsx` before attempting any fix.

3. **Didn't question why `--build` wasn't enough.** Docker's `--build` flag is widely assumed to give you a "fresh" container. It doesn't — it only rebuilds the image. Volume reuse is a separate concern and not obvious from the `docker compose up --help` output. Lesson: **anonymous volumes and `--build` are independent operations.**

## Changes Made

### 1. Entrypoint script (`scripts/docker-entrypoint-nextjs.sh`)

New file. Runs before `yarn dev` on every container start:

```
1. Hash current /app/yarn.lock (bind-mounted from host, always fresh)
2. Compare against saved hash in /app/node_modules/.yarn-lock-hash (persisted in volume)
3. If different → yarn install → save new hash
4. If same → skip (adds ~0 overhead to startup)
5. exec "$@" → hand off to CMD (yarn dev)
```

### 2. Dockerfile.dev updates (both apps)

```dockerfile
# Added: copy entrypoint into image
COPY scripts/docker-entrypoint-nextjs.sh /usr/local/bin/docker-entrypoint-nextjs.sh
RUN chmod +x /usr/local/bin/docker-entrypoint-nextjs.sh

# Changed: CMD → ENTRYPOINT + CMD
ENTRYPOINT ["docker-entrypoint-nextjs.sh"]
CMD ["yarn", "dev"]
```

### 3. docker-compose.yml updates (both apps)

```yaml
# Added: bind-mount yarn.lock so entrypoint can read host's current version
- ./yarn.lock:/app/yarn.lock:ro
```

### 4. Cleanup

- Removed unnecessary symlinks from `packages/admin-app/node_modules/` (clsx, class-variance-authority, tailwind-merge, lucide-react, radix-ui)
- Kept `outputFileTracingRoot` in `next.config.ts` (legitimate monorepo setting for production builds, harmless in dev)

## How the Fix Works

```
Before (broken):
  yarn add new-package → yarn.lock changes on host
  docker compose up -d --build → image rebuilt with new deps
  Container starts → mounts OLD anonymous volume → Can't resolve 'new-package'
  Developer must remember: docker compose down -v

After (self-healing):
  yarn add new-package → yarn.lock changes on host
  docker compose up -d --build → image rebuilt with new deps
  Container starts → entrypoint compares yarn.lock hashes → MISMATCH
  → yarn install runs inside container → volume updated → deps resolve
  Next restart → hashes match → instant startup (no install)
```

The fix is **additive** — it doesn't change the anonymous volume pattern (which still correctly prevents host `node_modules` from leaking into the container). It just adds a self-healing mechanism on top.

## Verification

| Check | Result |
|-------|--------|
| Entrypoint detects fresh volume (no hash file) | ✅ Runs `yarn install`, saves hash |
| Entrypoint detects matching hash | ✅ Skips install, prints "up to date" |
| Entrypoint detects changed yarn.lock | ✅ Runs `yarn install`, saves new hash |
| `exec "$@"` passes CMD correctly | ✅ `yarn dev` starts after entrypoint |
| `:ro` mount prevents container from modifying host yarn.lock | ✅ Read-only flag confirmed |
| Both admin-app and web-app use same entrypoint | ✅ Shared script |

## Recovery Playbook

The entrypoint should handle this automatically going forward. If it doesn't:

```bash
# Step 1: Check if entrypoint ran
docker compose logs admin-app | head -5
# Should show "Dependencies changed..." or "Dependencies up to date"

# Step 2: If entrypoint isn't running, verify ENTRYPOINT is set
docker inspect rooteditorial_admin_app --format='{{.Config.Entrypoint}}'
# Should show: [docker-entrypoint-nextjs.sh]

# Step 3: Nuclear option (same as before, but shouldn't be needed)
docker compose down admin-app -v
docker compose build --no-cache admin-app
docker compose up -d admin-app
```

## Open Questions

1. **Should `.next` also get hash-based invalidation?** The `.next` anonymous volume preserves Turbopack's compilation cache. Unlike `node_modules`, Turbopack manages its own cache invalidation, so stale `.next` shouldn't cause hard failures. But the Turbopack CPU loop may have been caused by corrupted `.next` state. Worth monitoring.

2. **Should `package.json` changes also trigger reinstall?** Currently only `yarn.lock` is compared. In Yarn, `package.json` changes always produce `yarn.lock` changes, so this should be sufficient. But if someone manually edits `yarn.lock` (don't do this) the detection would still work.

3. **Performance of `yarn install` on stale volume.** When the entrypoint detects a mismatch, `yarn install` runs against the existing volume contents. Yarn is smart enough to do a diff install (only add/remove changed packages), so this should be fast (~5-10 seconds). But on a completely empty volume (first container start), it's a full install (~30 seconds).

## What Went Well

- User recognized the pattern ("this isn't the first time") and pushed for a systemic fix instead of accepting the one-off `docker compose down -v` workaround
- Once the Docker runtime was identified, diagnosis was fast (10 minutes from "it's Docker" to "anonymous volumes are the cause")
- Fix is backwards-compatible — existing `dev.sh` workflows need no changes

## What Could Be Better

- `CLAUDE.md` should document that `dev.sh` (Docker) is the primary dev workflow, not bare `yarn dev`
- The anonymous volume pattern is a known Docker footgun but was undocumented in the project. New contributors would hit the same issue
- Both previous sessions debugging this project (Turbopack CPU loop, this incident) involved anonymous volume state. Consider whether named volumes with explicit lifecycle management would be clearer

## Follow-up (2026-03-11): Removed `--immutable` and `:ro`

The original fix used `yarn install --immutable` with `yarn.lock:ro`. This turned out to be a partial fix that caused its own class of failures:

- `--immutable` fails when Yarn's resolution differs between host (macOS) and container (Alpine Linux), even when the logical dependency tree is identical. Platform-specific optional deps like `sharp` trigger this.
- When `--immutable` failed, the entrypoint silently fell back to whatever was already in the volume — which could be completely stale from a previous image build.
- The `:ro` mount prevented `yarn install` (without `--immutable`) from working as a fallback.

**Changes made (2026-03-11):**
- `scripts/docker-entrypoint-nextjs.sh`: replaced `yarn install --immutable` with `yarn install`
- `docker-compose.yml`: removed `:ro` from admin-app's `yarn.lock` mount
- `app/globals.css`: inlined `shadcn/tailwind.css` contents to avoid CSS import resolution issues in Docker

See [Docker Architecture](../architecture/DOCKER_ARCHITECTURE.md) for the full explanation.

## Stats Summary

| Metric | Value |
|--------|-------|
| Time spent investigating | ~2 hours (across two compacted sessions) |
| Files created | 1 (`scripts/docker-entrypoint-nextjs.sh`) |
| Files modified | 3 (`docker-compose.yml`, `packages/admin-app/Dockerfile.dev`, `packages/web-app/Dockerfile.dev`) |
| Files cleaned up | 5 symlinks removed from `packages/admin-app/node_modules/` |
| Root cause | Docker anonymous volumes persist stale `node_modules` across image rebuilds |
| Fix type | Self-healing entrypoint with lockfile hash comparison |
| Services protected | 2 (admin-app, web-app) |
