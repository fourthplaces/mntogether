# Root Editorial

An open CMS for community journalism. Editors curate civic content — stories, events, resources, calls for help, directories — into a weekly broadsheet-style digital edition for every Minnesota county.

This is one half of a larger system:

```
┌─────────────────────────────────────────┐
│  Root Signal                            │   Scouts and extracts civic
│  (separate repo, upstream)              │   content. Produces fully-
│                                         │   formed post candidates.
└──────────────────┬──────────────────────┘
                   │  HTTP push
                   ▼
┌─────────────────────────────────────────┐
│  Root Editorial                         │   Curates, edits, sequences,
│  (this repo)                            │   publishes. Generates weekly
│                                         │   per-county broadsheets.
└──────────────────┬──────────────────────┘
                   │  serves
                   ▼
┌─────────────────────────────────────────┐
│  Public broadsheet (mntogether.org)     │   Readers.
└─────────────────────────────────────────┘
```

**Root Editorial does not crawl or extract.** It is strictly the consumer of Root Signal's output plus the human-curation and layout layer on top. The integration contract is specified in [`docs/handoff-root-signal/`](docs/handoff-root-signal/README.md).

---

## What's in this repo

- **Admin CMS** — per-post editing (Plate.js WYSIWYG), kanban-style review flow, Signal Inbox for triaging incoming posts, editor dashboard, per-county edition composer, revision chain tracking.
- **Broadsheet layout engine** — generates 3-column newspaper-style weekly editions for each of Minnesota's 87 counties plus a Statewide pseudo-county. Layout templates pick per post based on weight (heavy / medium / light) and post type.
- **9-type post taxonomy** — `story`, `update`, `action`, `event`, `need`, `aid`, `person`, `business`, `reference`. Each type carries its own required field groups (datetime, contacts, items, schedule, media, link, person, status).
- **Source graph** — organisation and individual source dedup, `post_sources` polymorphic linkage, idempotency-key handling on the ingest endpoint.
- **Public broadsheet site** — reader-facing county-scoped editions, individual post detail pages, county picker, Statewide fallback.
- **Root Signal ingest specification** — the [`docs/handoff-root-signal/`](docs/handoff-root-signal/README.md) package documents the integration contract Root Signal builds against.

---

## Quick start

```bash
cp .env.example .env
# Minimum for local dev: set JWT_SECRET, TEST_IDENTIFIER_ENABLED=true
./dev.sh
```

`./dev.sh` brings up the full stack (PostgreSQL, MinIO, Rust server, Admin app, Web app) in Docker Compose and opens a live-status dashboard. First run pulls images and compiles the Rust server — allow a few minutes. Keys: `[1]` open admin, `[2]` open web, `[l]` tail logs, `[b]` rebuild server, `[q]` quit.

**What's running after startup:**

| Service | URL | Role |
|---|---|---|
| Admin CMS | http://localhost:3000 | Editors curate here |
| Public web | http://localhost:3001 | Public-facing broadsheet |
| Rust server | :9080 | HTTP/JSON API + SSE |
| PostgreSQL | :5432 | Primary database |
| MinIO | :9000 (API), :9001 (console) | S3-compatible media storage |

**Test login** (with `TEST_IDENTIFIER_ENABLED=true`): phone `+1234567890`, any code (Twilio verification is skipped in dev).

Detailed setup notes in [`docs/setup/QUICK_START.md`](docs/setup/QUICK_START.md) and [`docs/setup/LOCAL_DEV_SETUP.md`](docs/setup/LOCAL_DEV_SETUP.md).

---

## Architecture

```
┌──────────────────┐     ┌──────────────────┐
│   Admin App      │     │   Web App        │
│  (Next.js CMS)   │     │  (Next.js public)│
│   :3000          │     │   :3001          │
└────────┬─────────┘     └────────┬─────────┘
         │                        │
         └────────┬───────────────┘
                  ▼
         ┌────────────────────┐
         │  GraphQL Yoga      │   Schema + resolvers shared
         │  (in-process)      │   between admin + web apps
         └─────────┬──────────┘
                   ▼
         ┌────────────────────┐     ┌─────────────┐
         │  Rust Axum Server  │────▶│ PostgreSQL  │
         │  HTTP/JSON + SSE   │     │  (pgvector) │
         │  :9080             │     │  :5432      │
         └──────────┬─────────┘     └─────────────┘
                    │
                    ├─▶ MinIO / S3 (media)
                    └─▶ Twilio (SMS auth)
```

GraphQL resolvers in the Next.js apps are thin wrappers around the Rust server's HTTP/JSON endpoints; the schema is shared between apps via the `@rooteditorial/shared` workspace package. Business logic lives in the Rust server's activity functions (`packages/server/src/domains/*/activities/`).

---

## Workspace packages

```
packages/
├── server/          # Rust — Axum HTTP server (all business logic, SQL, auth)
├── admin-app/       # TypeScript — Next.js CMS admin panel
├── web-app/         # TypeScript — Next.js public broadsheet site
├── shared/          # TypeScript — shared GraphQL schema + resolvers
├── twilio-rs/       # Rust — Twilio Verify client wrapper
└── dev-cli/         # Rust — operator CLI (API-key issuance, data migrations)
```

## Technology stack

| Component | Technology |
|---|---|
| Server | Rust + Axum + sqlx (PostgreSQL) |
| Database | PostgreSQL 17 with pgvector |
| Admin + public sites | Next.js 16 (App Router) + React |
| GraphQL | graphql-yoga, in-process in each Next app |
| Rich-text editor | Plate.js |
| Auth | Twilio Verify (phone OTP) + JWT; machine-token `ServiceClient` for ingest |
| Media storage | MinIO (S3-compatible) |
| CSS | Tailwind + component CSS |

---

## Documentation

Full docs under [`docs/`](docs/README.md). Highlights:

**Architecture + design**
- [Root Editorial Pivot](docs/architecture/ROOT_EDITORIAL_PIVOT.md) — the pivot bible: what Root Editorial is (and isn't)
- [Post Type System](docs/architecture/POST_TYPE_SYSTEM.md) — the 9-type taxonomy
- [Domain Architecture](docs/architecture/DOMAIN_ARCHITECTURE.md) — models / activities / HTTP handlers structure
- [CMS System Spec](docs/architecture/CMS_SYSTEM_SPEC.md) — CMS and broadsheet spec

**Root Signal integration**
- [Handoff package](docs/handoff-root-signal/README.md) — specification for Root Signal engineers building the ingest integration
- [Data contract](docs/architecture/ROOT_SIGNAL_DATA_CONTRACT.md) — authoritative on-the-wire contract
- [Taxonomy expansion brief](docs/handoff-root-signal/TAXONOMY_EXPANSION_BRIEF.md) — argument for Profile / LocalBusiness / Opportunity signal types
- [Tag vocabulary](docs/handoff-root-signal/TAG_VOCABULARY.md) — topic, service_area, and safety reference

**Decisions + state**
- [Decisions log](docs/DECISIONS_LOG.md) — architectural decisions with context
- [Outstanding work](docs/TODO.md) — prioritised build queue

**Setup + ops**
- [Quick start](docs/setup/QUICK_START.md)
- [Local dev setup](docs/setup/LOCAL_DEV_SETUP.md)
- [Docker guide](docs/setup/DOCKER_GUIDE.md)
- [Deployment](docs/setup/DEPLOYMENT.md)

---

## Project status

Root Editorial is in active development. The schema, admin CMS, and broadsheet generation are production-ready; the system currently runs on seeded dummy content (168 hand-authored posts across 87 counties). The cutover to live data flows from Root Signal is the current critical path — tracked as item #1 in [`docs/TODO.md`](docs/TODO.md), specified in [`docs/handoff-root-signal/`](docs/handoff-root-signal/README.md).

---

## License

MIT.
