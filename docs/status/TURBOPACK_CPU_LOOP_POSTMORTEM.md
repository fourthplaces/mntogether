# Postmortem: Turbopack Infinite Compilation Loop

**Date**: 2026-02-26
**Duration**: ~3 hours of investigation
**Impact**: Admin app dev server consuming 250% CPU, "Compiling..." indicator stuck permanently
**Resolution**: Container restart (destroyed stale Turbopack in-memory state)
**Root cause**: Known class of Turbopack bugs in Next.js 16; specific trigger unknown

---

## Timeline

| Time | Event |
|------|-------|
| ~T+0 | User navigates to `localhost:3000/admin/dashboard`, sees "Compiling..." stuck in bottom-left |
| ~T+5m | Confirmed via `docker stats`: admin-app at **239% CPU, 803MB RAM** vs web-app at 0.01% |
| ~T+10m | First wrong fix: added `devIndicators: false` to suppress the badge. User correctly rejected this ("we should solve the root cause") |
| ~T+20m | Noticed `⚠ The "middleware" file convention is deprecated` in container logs. Hypothesized middleware.ts was the trigger |
| ~T+30m | Migrated `middleware.ts` → `proxy.ts` (Next.js 16 convention). CPU still 250% — **did not restart container**, so test was invalid |
| ~T+45m | Discovered key behavioral pattern: CPU drops to 0% instantly when browser navigates away, spikes when browser connects. The HMR WebSocket connection is required for the loop |
| ~T+60m | Ran `strace` inside container: ~0 syscalls during 250% CPU burn. Turbopack's native Rust code spinning in pure userspace — no file I/O, no network I/O, nothing observable |
| ~T+75m | Tested `next dev --webpack`: **0% CPU**. Confirmed Turbopack-specific |
| ~T+90m | Removed proxy.ts entirely + proper container restart → still 250%. Proved proxy/middleware file is irrelevant |
| ~T+100m | Ran `docker compose down admin-app && docker compose up -d admin-app` (destroys anonymous volume + creates new process) |
| ~T+105m | CPU at 0% with browser connected. Issue resolved |
| ~T+120m | User pushed for root cause certainty. Began systematic bisection |
| ~T+130m | **Test 1**: Restored original `middleware.ts` + fresh container → 0% CPU. **Middleware is not the trigger** |
| ~T+140m | Simulated rapid file mutations (create/delete middleware/proxy 5x) while dev server watched → brief 24% spike, recovered to 0%. **Cannot reproduce the loop** |
| ~T+150m | Checked for known triggers: no circular imports, no `opengraph-image.tsx`, no favicon 404, no dynamic `import()` calls |
| ~T+160m | Found multiple matching GitHub issues. Classified as known Turbopack bug |

## What Happened

The admin-app's Turbopack dev server entered an infinite compilation loop. When a browser established an HMR WebSocket connection, Turbopack's native Rust code (`next-swc`) began spinning at 250% CPU with zero observable I/O. The "Compiling..." indicator stayed permanently lit. The loop persisted across page navigations within the app and across different browser tabs, but stopped instantly when no browser was connected.

The web-app (identical Next.js version, identical PostCSS/Tailwind config, same Docker volume setup) was unaffected at 0% CPU.

## Investigation Results

### What we confirmed

| Hypothesis | Test | Result |
|-----------|------|--------|
| Stale `.next` cache | `docker compose down` + `up` (destroys anonymous volume) | ✅ Fixed it |
| `middleware.ts` deprecation | Restored middleware.ts + fresh container | ❌ 0% CPU — not the cause |
| File watcher race condition | Rapid create/delete of middleware/proxy files | ❌ Brief recompile, recovered fine |
| Circular imports | Full import graph analysis (50 files) | ❌ Clean unidirectional imports |
| opengraph-image.tsx trigger | Checked for file | ❌ Not present in either app |
| Missing favicon 404 | Captured all network requests on page load | ❌ All 200s |
| PostCSS infinite expansion | Compared `.next/dev/build/postcss.js` size | ❌ 899 bytes, identical to web-app |
| Dynamic `import()` cycles | Searched all TS/TSX files | ❌ No dynamic imports found |
| Webpack alternative | `next dev --webpack` | ✅ 0% CPU — Turbopack-specific |

### What we could not determine

1. **The specific trigger.** We could not reproduce the 250% loop despite trying file mutations, stress edits, and fresh containers with various configurations.

2. **Whether it was in-memory state or on-disk cache.** Our fix destroyed both simultaneously (container restart = new process + new anonymous volume). We could not isolate which one mattered.

3. **Whether it will recur.** Multiple GitHub reporters describe this as intermittent: "everything will run fine for a little bit, then, randomly, it will just hang."

## Known Turbopack Issues (Matching Symptoms)

| Issue | Description | Status |
|-------|-------------|--------|
| [vercel/next.js#87322](https://github.com/vercel/next.js/issues/87322) | Infinite "Compiling" loop with opengraph-image.tsx | Closed (specific trigger fixed) |
| [vercel/next.js#85119](https://github.com/vercel/next.js/issues/85119) | Dynamic cyclical import causes infinite loop | Open |
| [vercel/next.js#77102](https://github.com/vercel/next.js/discussions/77102) | Dev server stuck in compiling + extreme CPU/memory | Open (multiple reporters on 15.x–16.x) |
| [vercel/next.js#81161](https://github.com/vercel/next.js/issues/81161) | Turbopack dev server uses too much RAM and CPU | Open |

Key evidence from those threads:
- Thread stack traces show `next-swc.darwin-arm64.node` stuck in a **"notify-rs fsevents loop"** — the Rust file watcher spinning. Matches our strace findings (pure userspace CPU, zero syscalls).
- Vercel team acknowledged one trigger: *"There was one case we know of that caused an infinite loop with a specific combination of import() nesting. This has been fixed in 15.3."* But reporters on 15.3+ and 16.x still encounter it.
- One reporter found `.next/postcss.js` consuming 20GB+ of memory. Not our case (899 bytes) but shows PostCSS processing is another trigger.
- Multiple reporters confirm `--webpack` as workaround and container/process restart as fix.

## Mistakes During Investigation

1. **Suppressed the indicator first.** Added `devIndicators: false` before understanding the problem. User correctly pushed back. Lesson: symptoms are signals, don't hide them.

2. **Didn't restart container after removing middleware.ts.** Changed a file, tested, concluded "not the cause" — but Turbopack had already cached the bad state. The test was invalid. This cost ~20 minutes and led to a wrong conclusion.

3. **Changed two variables at once.** The fix (container restart) simultaneously cleared in-memory state AND the `.next` anonymous volume. We could not isolate which mattered.

4. **Applied `--webpack` to web-app during testing.** Changed the unaffected app's bundler as a "consistency" measure. Had to revert. Don't change things that aren't broken.

## Changes Made

### `middleware.ts` → `proxy.ts` migration

Although this didn't fix the CPU loop, it's the correct Next.js 16 convention and eliminates the deprecation warning:

```
- packages/admin-app/middleware.ts  (deleted)
+ packages/admin-app/proxy.ts      (created — same logic, function renamed from `middleware` to `proxy`)
```

The proxy.ts file handles:
- Redirecting `/admin` → `/admin/dashboard`
- Protecting admin routes behind JWT auth cookie
- Redirecting authenticated users away from `/admin/login`

Container logs confirm it works: `GET /admin/dashboard 200 in 94ms (compile: 4ms, proxy.ts: 12ms, render: 78ms)`

## Recovery Playbook

If the "Compiling..." loop recurs:

```bash
# Step 1: Confirm it's the loop (not just slow compilation)
docker stats --no-stream | grep admin
# If CPU > 100% for more than 30 seconds, it's the loop

# Step 2: Restart the container (clears in-memory state + .next volume)
docker compose down admin-app && docker compose up -d admin-app

# Step 3: If it recurs immediately, capture a Turbopack trace for a bug report
# Add to docker-compose.yml environment:
#   NEXT_TURBOPACK_TRACING: 1
# Restart, reproduce, then grab:
#   docker cp rooteditorial_admin_app:/app/packages/admin-app/.next/trace-turbopack ./
# File the trace at: https://github.com/vercel/next.js/issues

# Step 4: Nuclear option — switch to webpack
# In packages/admin-app/package.json:
#   "dev": "next dev --webpack"
```

## Open Questions

1. **What accumulates in Turbopack's state to trigger the loop?** The container had been running for hours through a development session before the user noticed. Is it time-based? HMR-cycle-count-based? Specific file-change-pattern-based?

2. **Why only admin-app?** Both apps have identical Next.js/Tailwind/PostCSS configs. Admin-app has route groups `(app)`/`(auth)`, proxy.ts, and more files (50 vs 31), but none of these individually trigger the bug. It may be a combination, or it may be random.

3. **Will upgrading Next.js help?** The Vercel team has fixed specific triggers across 15.3, 15.4, and 16.x. Upgrading when a new patch drops is worth trying, but the class of bugs has multiple unfixed triggers.
