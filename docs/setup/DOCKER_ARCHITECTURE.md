# Docker Development Architecture

How the Docker dev environment works, why it's built this way, and what to do when things go wrong.

## The Core Problem

This is a Yarn workspaces monorepo with three packages (`admin-app`, `web-app`, `shared`) running inside Docker containers. Two tensions drive the design:

1. **Source code must be editable from the host** (for IDE support and hot reload) — so we bind-mount `./packages/admin-app:/app/packages/admin-app`.
2. **`node_modules` must live inside the container** — because native binaries (sharp, etc.) are compiled for Linux/Alpine, not macOS.

If we bind-mounted source code without protecting `node_modules`, the host's macOS `node_modules` would overwrite the container's Linux `node_modules`. Named volumes solve this by overlaying container-owned directories on top of the bind mount.

## Volume Strategy

```yaml
# docker-compose.yml — admin-app volumes
volumes:
  - ./packages/admin-app:/app/packages/admin-app   # Source code (host → container)
  - ./packages/shared:/app/packages/shared          # Shared package
  - ./yarn.lock:/app/yarn.lock                      # Lockfile (host → container, read-write)
  - admin_node_modules:/app/node_modules            # Container-owned (named volume)
  - admin_app_node_modules:/app/packages/admin-app/node_modules  # Container-owned
  - admin_next_cache:/app/packages/admin-app/.next  # Turbopack cache
```

### Why named volumes, not anonymous volumes

We switched from anonymous volumes to named volumes early on. Anonymous volumes are tied to container identity — `docker compose down` without `-v` leaves orphaned volumes, and `-v` destroys all of them (including the `.next` cache). Named volumes have explicit lifecycle: they persist across `docker compose down` and can be individually removed with `docker volume rm`.

### The stale volume problem

Named volumes persist across image rebuilds. When you `yarn add` a new package on the host:

1. `yarn.lock` updates on the host
2. `docker compose up --build` rebuilds the image (fresh `yarn install` inside)
3. Container starts — but the **named volume** mounts over `/app/node_modules` with the **old** contents

The image has the right deps, but the volume shadows them. This is the single most common Docker issue we've hit. See the [stale deps postmortem](../status/DOCKER_STALE_DEPS_POSTMORTEM.md) for the full investigation.

### The entrypoint fix

`scripts/docker-entrypoint-nextjs.sh` runs before `yarn dev` on every container start:

```
1. SHA256 hash current /app/yarn.lock (bind-mounted from host)
2. Compare against saved hash in /app/node_modules/.yarn-lock-hash (in the volume)
3. Mismatch → yarn install → save new hash (volume updated, ~5-10s)
4. Match → skip (instant startup)
5. exec "$@" → hand off to CMD (yarn dev)
```

Both `admin-app` and `web-app` share this entrypoint.

### Evolution of the entrypoint

| Version | Approach | Problem |
|---------|----------|---------|
| v1 (Feb 2026) | `yarn install --immutable` with `yarn.lock:ro` mount | `--immutable` fails when Yarn's resolution differs even slightly between host and container. `:ro` mount prevents recovery. Entrypoint silently falls back to stale image deps. |
| v2 (Mar 2026) | `yarn install` with `yarn.lock` read-write mount | Just works. Yarn reads the lockfile and installs from it. If resolution produces lockfile changes, they propagate back to the host via the bind mount — keeping host and container in sync. |

**Why `--immutable` was wrong for dev**: `--immutable` is designed for CI — it enforces that the lockfile exactly matches what `yarn install` would produce, failing otherwise. In a Docker container with a different OS/architecture, Yarn may resolve optional dependencies differently (e.g., platform-specific `sharp` binaries), causing `--immutable` to fail even when the logical dependency tree is identical. The failure mode was silent: the entrypoint caught it, logged a warning, and continued with whatever was in the volume — which could be completely stale.

**Why `:ro` was wrong**: The read-only mount was added to prevent the container from "corrupting" the host's lockfile. But `yarn install` reading a lockfile doesn't corrupt it — it installs exactly what the lockfile specifies. And if the lockfile does need updating (e.g., platform-specific resolutions), having that change reflected on the host is actually desirable. The `:ro` mount turned a self-healing mechanism into a failure.

## Build Pipeline

### Image build (`docker compose build` / `docker compose up --build`)

The Dockerfiles follow the same pattern for both Next.js apps:

```dockerfile
FROM node:22-alpine
WORKDIR /app
RUN corepack enable && corepack prepare yarn@4.12.0 --activate

# Copy workspace root + package manifests (cache-friendly layer)
COPY package.json yarn.lock .yarnrc.yml ./
COPY packages/shared ./packages/shared
COPY packages/admin-app/package.json ./packages/admin-app/

# Install deps (baked into image)
RUN yarn install

# Copy entrypoint
COPY scripts/docker-entrypoint-nextjs.sh /usr/local/bin/docker-entrypoint-nextjs.sh
RUN chmod +x /usr/local/bin/docker-entrypoint-nextjs.sh

WORKDIR /app/packages/admin-app
ENTRYPOINT ["docker-entrypoint-nextjs.sh"]
CMD ["yarn", "dev"]
```

**Layer caching**: `package.json` and `yarn.lock` are copied before source code. If only source code changes, Docker skips the `yarn install` layer entirely. The entrypoint handles the case where deps changed but the volume is stale.

**No source code in the image**: Source code is bind-mounted at runtime, not baked into the image. The image only contains `node_modules` and the entrypoint. This means `docker compose build` only needs to run when dependencies change, not on every code edit.

### Container startup flow

```
docker compose up -d admin-app
  │
  ├─ Docker mounts volumes (source, yarn.lock, named volumes)
  │
  ├─ ENTRYPOINT: docker-entrypoint-nextjs.sh
  │   ├─ Hash yarn.lock
  │   ├─ Compare to saved hash in volume
  │   ├─ Mismatch? → yarn install → save hash
  │   └─ Match? → skip
  │
  └─ CMD: yarn dev
      └─ Next.js dev server with Turbopack (HMR via bind mount)
```

### The Rust server

The server follows a different pattern — no entrypoint script, no volume hash checking:

```yaml
server:
  volumes:
    - ./Cargo.toml:/app/Cargo.toml
    - ./Cargo.lock:/app/Cargo.lock
    - ./packages/server:/app/packages/server
    - rust_target:/app/target        # Named volume for build cache
  command: cargo watch -w /app/packages/server -s 'cargo run --bin server'
```

Rust deps are compiled into `/app/target` (named volume). `cargo watch` handles rebuilds automatically. Unlike Node.js, Cargo doesn't need a separate volume-sync mechanism because `cargo build` always reads `Cargo.lock` fresh and rebuilds as needed — the build tool itself is the sync mechanism.

## Common Operations

### Adding a new npm dependency

```bash
# On host:
cd packages/admin-app && yarn add some-package

# Then either:
docker compose restart admin-app     # Entrypoint detects yarn.lock change, runs yarn install
# or:
docker compose up -d --build admin-app  # Rebuilds image AND entrypoint syncs volume
```

Both approaches work. `restart` is faster (~10s for yarn install) vs `--build` (~30s for full image rebuild + install).

### Clearing stale state

If something is truly broken and you need a clean slate:

```bash
# Nuclear option — removes named volumes, forces fresh install
docker compose down admin-app
docker volume rm rooteditorial_admin_node_modules rooteditorial_admin_app_node_modules
docker compose up -d --build admin-app
```

The `.next` cache volume (`rooteditorial_admin_next_cache`) can also be removed if Turbopack is misbehaving, but this is rare.

### Checking entrypoint status

```bash
docker compose logs admin-app | head -5
# Should show one of:
#   "⚡ Dependencies changed — running yarn install..."
#   "✅ Dependencies up to date (yarn.lock unchanged)."
```

## Known Issues and Trade-offs

### Trade-off: Named volumes vs fresh installs

Named volumes mean `yarn install` runs against existing `node_modules` contents (a diff install). This is fast (~5-10s) but means the volume accumulates cruft over time — removed packages leave behind their directories until a clean `yarn install` runs. In practice this doesn't cause issues because Node.js resolution only looks at what's in `package.json`, not what's on disk.

The alternative — anonymous volumes or no volumes with `yarn install` on every start — would be slower (~30s) but guaranteed clean. We chose speed.

### Trade-off: Shared entrypoint script

Both Next.js apps use the same `scripts/docker-entrypoint-nextjs.sh`. This keeps things DRY but means app-specific startup logic would need conditionals. So far this hasn't been needed.

### Known: Turbopack `.next` cache

The `.next` named volume preserves Turbopack's compilation cache across restarts. Unlike `node_modules`, Turbopack manages its own cache invalidation — it watches files and recompiles as needed. The one known failure mode is corrupted cache state (see [Turbopack CPU Loop Postmortem](../status/TURBOPACK_CPU_LOOP_POSTMORTEM.md)), which requires removing the volume.

### Known: First Rust compile is slow

The first `cargo build` after a fresh `rust_target` volume takes 5-10 minutes. Subsequent builds are fast thanks to incremental compilation cached in the volume.

## Service Dependency Graph

```
                ┌─────────┐
                │ postgres │ (healthcheck: pg_isready)
                └────┬────┘
                     │
              ┌──────┴──────┐
              │  minio-init │ (runs once, creates bucket)
              └──────┬──────┘
                     │
                ┌────┴────┐
                │  server  │ (waits for postgres healthy + minio-init complete)
                └────┬────┘
                     │
           ┌─────────┴─────────┐
           │                   │
      ┌────┴────┐        ┌────┴────┐
      │admin-app│        │ web-app │
      └─────────┘        └─────────┘
```

Ports: postgres:5432, minio:9000/9001, server:9080, admin-app:3000, web-app:3001
