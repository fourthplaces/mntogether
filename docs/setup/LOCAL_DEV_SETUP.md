# Local Development Setup & Test Data

## Quick Start (from scratch)

```bash
# 1. Clone and enter repo
gh repo clone fourthplaces/mntogether
cd mntogether

# 2. Generate Cargo.lock (gitignored)
cargo generate-lockfile

# 3. Set up environment
cp .env.example .env
# Edit .env:
#   JWT_SECRET=<generate with: openssl rand -base64 32>
#   TEST_IDENTIFIER_ENABLED=true
#   PII_SCRUBBING_ENABLED=false
# API keys (OpenAI, Twilio, etc.) are NOT needed for design/frontend work

# 4. Start services
docker compose up -d
# First build takes 5-10 min (Rust compile)

# 5. Run migrations
docker compose exec server sqlx migrate run --source /app/packages/server/migrations

# 6. Restore test data (if available)
docker compose exec -T postgres psql -U postgres -d rooteditorial < data/local_test_db.sql
```

## Restoring the Test Database

A snapshot of the test database is saved at:

- `data/local_test_db.sql` (283K, full pg_dump)
- `data/local_test_db.sql.gz` (71K, compressed)

To restore:

```bash
# Drop and recreate (if you want a clean slate)
docker compose exec postgres psql -U postgres -c "DROP DATABASE rooteditorial; CREATE DATABASE rooteditorial;"
docker compose exec -T postgres psql -U postgres -d rooteditorial < data/local_test_db.sql

# Or just restore on top of a fresh migration
docker compose exec server sqlx migrate run --source /app/packages/server/migrations
docker compose exec -T postgres psql -U postgres -d rooteditorial < data/local_test_db.sql
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

### GraphQL API via Restate

The frontend talks to the Rust server through the Restate runtime, which provides durable workflow execution:

```
Browser --> Next.js App (3000/3001) --> Restate Runtime (8180) --> Rust Server (9080)
```

The admin-app and web-app both communicate with the backend via GraphQL, with the shared package defining the schema types.

### Filter queries join through tags

The public post listing filters by joining posts to tags:
- `post_type` filter: joins `taggables` + `tags WHERE kind = 'post_type'`
- `category` filter: joins `taggables` + `tags WHERE kind = 'service_offered'`

### Public tag display

The `find_public_for_post_ids` query joins `tags.kind` to `tag_kinds.slug` and filters `tag_kinds.is_public = true`. Only tags whose kind has `is_public = true` appear on the public post cards.

## Services & Ports

| Service | Port | Notes |
|---|---|---|
| Admin App (Next.js) | 3000 | CMS admin panel |
| Web App (Next.js) | 3001 | Public site |
| Rust Server | 9080 | Restate workflow services |
| Restate Runtime | 8180 (ingress), 9070 (admin) | Workflow orchestration |
| PostgreSQL | 5432 | pgvector (see docker-compose.yml for credentials) |

## Test Auth

With `TEST_IDENTIFIER_ENABLED=true`, log in with:
- Phone: `+1234567890`
- Code: any value (Twilio verification is skipped)

No Twilio API keys needed.

## Gotchas

1. **Cargo.lock is gitignored** - Run `cargo generate-lockfile` after cloning
2. **First Rust compile is slow** - 5-10 min inside Docker on first `docker compose up`
3. **The `post_type` column on posts is NOT used for filtering** - Filtering uses tags (kind = 'post_type')
4. **Docker volumes persist across restarts** - Data survives `docker compose down`. Only `docker compose down -v` or `make clean` wipes data.
