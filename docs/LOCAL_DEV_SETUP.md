# Local Development Setup & Test Data

## Quick Start (from scratch)

```bash
# 1. Clone and enter repo
gh repo clone fourthplaces/mntogether
cd mntogether

# 2. Generate Cargo.lock (gitignored)
cargo generate-lockfile

# 3. Create .yarn directory for web package (gitignored)
mkdir -p packages/web/.yarn

# 4. Set up environment
cp .env.example .env
# Edit .env:
#   JWT_SECRET=<generate with: openssl rand -base64 32>
#   TEST_IDENTIFIER_ENABLED=true
#   PII_SCRUBBING_ENABLED=false
#   PII_USE_GPT_DETECTION=false
# API keys (OpenAI, Twilio, etc.) are NOT needed for design/frontend work

# 5. Start services
docker compose up -d
# First build takes 5-10 min (Rust compile)

# 6. Run migrations
docker compose exec server sqlx migrate run --source /app/packages/server/migrations

# 7. Restore test data (if available)
docker compose exec -T postgres psql -U postgres -d mndigitalaid < data/local_test_db.sql
```

## Restoring the Test Database

A snapshot of the test database is saved at:

- `data/local_test_db.sql` (283K, full pg_dump)
- `data/local_test_db.sql.gz` (71K, compressed)

To restore:

```bash
# Drop and recreate (if you want a clean slate)
docker compose exec postgres psql -U postgres -c "DROP DATABASE mndigitalaid; CREATE DATABASE mndigitalaid;"
docker compose exec -T postgres psql -U postgres -d mndigitalaid < data/local_test_db.sql

# Or just restore on top of a fresh migration
docker compose exec server sqlx migrate run --source /app/packages/server/migrations
docker compose exec -T postgres psql -U postgres -d mndigitalaid < data/local_test_db.sql
```

## What the Test Data Contains

A snapshot of real posts was scraped from the live site for local testing. The actual data lives in `data/local_test_db.sql` — see [Restoring the Test Database](#restoring-the-test-database) above.

### Posts (25)

All posts have `status = 'active'` and `post_type = 'opportunity'` in the DB column. The actual user-facing post type filtering uses **tags**, not the column (see Tag System below).

Content covers food assistance and community advocacy topics.

### Organizations (3)

- 3 organizations with 5–6 posts each
- 10 posts have no organization (unaffiliated community content)

### How Posts Link to Organizations

Posts do NOT have a direct FK to organizations. The relationship is:

```
posts --> post_sources --> sources --> organizations
                          (has organization_id FK)
```

The backend resolves `organization_name` by joining through this chain. When creating test data, you must create entries in all three tables.

### Tag System

Tags are the primary mechanism for filtering and display. Key concepts:

- `tags` table: `(kind, value)` is unique. `kind` maps to `tag_kinds.slug`.
- `taggables` table: polymorphic join (`taggable_type = 'post'`, `taggable_id = post.id`).
- `tag_kinds` table: controls which tags appear publicly (`is_public = true`).

#### Tag kinds that matter for the public site:

| tag_kinds.slug | is_public | Purpose |
|---|---|---|
| `public` | true | User-visible badges (Donate, Volunteer, Food, Help) |
| `post_type` | false | Post type filter tabs (offering, seeking, announcement) |
| `service_offered` | false | Category filter dropdown (food-assistance, housing, legal-aid) |

#### Current tag distribution:

| Kind | Value | Display Name | Posts |
|---|---|---|---|
| post_type | offering | Support | 14 |
| post_type | seeking | Help | 2 |
| service_offered | food-assistance | Food Assistance | 10 |
| service_offered | housing | Housing | 1 |
| service_offered | legal-aid | Legal Aid | 1 |
| public | Donate | Donate | 2 |
| public | Volunteer | Volunteer | 1 |
| public | food assistance | Food | 1 |
| public | Help | Help | 1 |

### Contacts (32)

Stored in `listing_contacts` (not `contacts`). Types: phone, email, website. Linked to posts via `listing_id` FK.

### Schedules (26)

Stored in `schedules` with polymorphic `schedulable_type = 'post'`. Contains day_of_week, opens_at, closes_at, timezone.

## Architecture Notes

### API is Restate, not GraphQL

The frontend talks to Restate RPC endpoints via a Next.js proxy:

```
Browser --> /api/restate/Posts/public_list --> Restate Runtime (8180) --> Server (9080)
```

Key public endpoints (no auth needed):
- `POST /api/restate/Posts/public_list` - Paginated post list with optional `post_type` and `category` filters
- `POST /api/restate/Posts/public_filters` - Available filter options with counts
- `POST /api/restate/Post/{id}/get` - Single post detail with contacts, schedules, tags, org info

### Filter queries join through tags

The `public_list` endpoint filters by joining posts to tags:
- `post_type` filter: joins `taggables` + `tags WHERE kind = 'post_type'`
- `category` filter: joins `taggables` + `tags WHERE kind = 'service_offered'`

The `public_filters` endpoint returns available filter options:
- Post types: hardcoded to `offering`, `seeking`, `announcement` from `tags WHERE kind = 'post_type'`
- Categories: dynamic from `tags WHERE kind = 'service_offered'` joined to active posts

### Public tag display

The `find_public_for_post_ids` query joins `tags.kind` to `tag_kinds.slug` and filters `tag_kinds.is_public = true`. Only tags whose kind has `is_public = true` appear on the public post cards.

## Services & Ports

| Service | Port | Notes |
|---|---|---|
| Next.js Web | 3000 | Public site + admin |
| Rust Server | 9080 | Restate services |
| SSE Server | 8081 | Real-time streaming |
| Restate Runtime | 8180 (ingress), 9070 (admin) | Workflow orchestration |
| PostgreSQL | 5432 | pgvector (see docker-compose.yml for credentials) |
| Redis | 6379 | Job queue, pub/sub |
| NATS | 4222 | Messaging |

## Test Auth

With `TEST_IDENTIFIER_ENABLED=true`, log in with:
- Email: `test@example.com`
- Code: `123456`

No Twilio API keys needed.

## Gotchas

1. **Cargo.lock is gitignored** - Run `cargo generate-lockfile` after cloning
2. **packages/web/.yarn/ is gitignored** - Create it with `mkdir -p packages/web/.yarn`
3. **Port 4222 conflict** - If NATS fails to start, check for another NATS container: `docker ps -a --filter publish=4222`
4. **First Rust compile is slow** - 5-10 min inside Docker on first `docker compose up`
5. **The `post_type` column on posts is NOT used for filtering** - Filtering uses tags (kind = 'post_type')
6. **`seed_organizations` binary needs OpenAI key** - Use the SQL dump instead for offline setup
7. **Docker volumes persist across restarts** - Data survives `docker compose down`. Only `docker compose down -v` or `make clean` wipes data.
