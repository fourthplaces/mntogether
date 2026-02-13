# Frontend Architecture (Separate Services)

> **Note**: The frontends are NO LONGER embedded in the server binary. They run as separate services. This document describes the current architecture.

## Architecture

The server and frontends run independently:

```
packages/web/          → Next.js App Router (port 3000)
packages/admin-app/    → Next.js Admin Panel
packages/server/       → Rust Restate Server (port 9080) + SSE (port 8081)
```

The `build.rs` in `packages/server/` confirms this:
```rust
// Frontend apps are now built and served separately
// via Docker Compose or standalone, not embedded in the server binary
fn main() {
    println!("cargo:warning=Frontend apps are served separately (see docker-compose.yml)");
}
```

## Services

### Web App (`packages/web/`)
- **Framework**: Next.js with App Router
- **Port**: 3000
- **Purpose**: Public-facing web app (browse posts, view organizations, chat)
- **Communicates with**: Restate runtime (port 9070) via HTTP, SSE server (port 8081) for real-time

### Admin Panel (`packages/admin-app/`)
- **Framework**: Next.js
- **Purpose**: Content moderation (approve/reject posts, manage sources, review proposals)
- **Protected**: Yes (JWT authentication)

### Rust Server (`packages/server/`)
- **Port 9080**: Restate workflow endpoint (services, workflows, virtual objects)
- **Port 8081**: SSE server for real-time streaming events
- **No frontend serving** — the server does not serve any static assets

### Shared (`packages/shared/`)
- GraphQL schema definitions
- Shared TypeScript types
- Used by both web and admin-app

## Communication Flow

```
Web App (3000) ──→ Restate Runtime (9070) ──→ Rust Server (9080)
     ↑                                              │
     └──────────── SSE (8081) ←─────────────────────┘
```

1. **Web app** makes HTTP calls to Restate runtime for data operations
2. **Restate runtime** routes to the appropriate service/workflow/object
3. **SSE server** pushes real-time updates back to the frontend

## Development Workflow

### Separate Dev Servers (Recommended)

```bash
# Terminal 1: Infrastructure
docker-compose up -d postgres redis nats restate

# Terminal 2: Rust server
cd packages/server
cargo run --bin server
# Listening on ports 9080 (Restate) and 8081 (SSE)

# Terminal 3: Web app
cd packages/web
yarn dev
# Visit: http://localhost:3000

# Terminal 4: Admin panel (if needed)
cd packages/admin-app
yarn dev
```

### Docker Compose

For production-like setup, use Docker Compose which runs all services together.

## Historical Note

The server previously embedded both frontends using `rust-embed`, serving them from the binary at `/` (web app) and `/admin` (admin panel). This was replaced with separate services for faster development iteration and standard Next.js deployment.
