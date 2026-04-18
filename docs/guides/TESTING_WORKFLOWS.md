# Testing Guide

Test-driven development and API-edge testing for the Rust server.

> **Filename note**: this file is named `TESTING_WORKFLOWS.md` for
> historical reasons — the project once ran on Restate workflows. The
> Restate runtime is gone; the server is now a plain Axum HTTP service
> and testing happens at the HTTP handler boundary. Content below
> reflects the current state.

## Architecture

```
┌─────────────────────┐
│  Next.js Apps       │  :3000 (admin), :3001 (web)
│  (GraphQL clients)  │
└──────────┬──────────┘
           │ HTTPS + GraphQL
           ↓
┌─────────────────────┐
│  GraphQL resolvers   │  (in-process in Next.js API routes)
└──────────┬──────────┘
           │ HTTP/JSON
           ↓
┌─────────────────────┐
│  Rust Server        │  :9080 (Axum)
│  (Activities +       │
│   handlers)          │
└──────────┬──────────┘
           │
           ↓
┌─────────────────────┐
│  PostgreSQL         │  :5432
└─────────────────────┘
```

## Test discipline (must-read)

See [CLAUDE.md](../../CLAUDE.md) for the full rules. Summary:

- **TDD is mandatory.** RED (failing test) → GREEN (simplest pass) →
  REFACTOR. No code without a test.
- **Test only at API edges.** Go through Axum HTTP handlers or
  GraphQL endpoints. Verify via API response *and* model queries.
  Never bypass with direct DB writes in tests.
- **Use models, not raw SQL in tests.** `Post::find_by_id(...)` not
  `sqlx::query!(...)`.
- **External services are mocked via `TestDependencies`.**
- **One assertion per test.** Failure messages should tell you what
  broke without reading the test body.

## Quick start

```bash
# Start infrastructure
docker compose up -d

# Run the full Rust test suite
cargo test

# Run one test file
cargo test --test post_creation_tests

# Run one test by name
cargo test --test post_creation_tests -- submit_post_creates_active_post
```

Tests auto-spin up and tear down a test database per `#[test_context]`
harness invocation — they don't touch your dev seed data.

## Test file layout

```
packages/server/tests/
├── common/            # Shared TestHarness + helpers
├── post_*.rs          # Post domain tests
├── edition_*.rs       # Edition domain tests
├── note_*.rs          # Notes domain tests
└── ...
```

Naming: `{feature}_tests.rs`, one file per feature area.

## Test harness skeleton

```rust
use test_context::test_context;
use crate::common::TestHarness;

#[test_context(TestHarness)]
#[tokio::test]
async fn update_post_changes_title(harness: &mut TestHarness) -> Result<()> {
    // Arrange — create a post via the API
    let post_id = harness
        .call("Post", "admin_create", json!({ "title": "Original", ... }))
        .await?
        .get_id();

    // Act — hit the API edge we're testing
    let res = harness
        .call(
            "Post",
            &format!("{post_id}/update_content"),
            json!({ "title": "Updated" }),
        )
        .await?;

    // Assert — API response
    assert_eq!(res["title"], "Updated");

    // Verify — model state
    let post = Post::find_by_id(post_id, &harness.pool).await?.unwrap();
    assert_eq!(post.title, "Updated");

    Ok(())
}
```

## Manual smoke tests

For ad-hoc exploration against a running local server (`docker compose up`):

```bash
# Health check
curl http://localhost:9080/health

# Send OTP (test mode — skips Twilio)
curl -X POST http://localhost:9080/Auth/send_otp \
  -H "Content-Type: application/json" \
  -d '{"phone_number":"+1234567890"}'

# Verify OTP (test mode — accepts any code)
curl -X POST http://localhost:9080/Auth/verify_otp \
  -H "Content-Type: application/json" \
  -d '{"phone_number":"+1234567890","code":"000000"}'
```

Test mode is enabled when `TEST_IDENTIFIER_ENABLED=true` is set in
`.env`. Twilio verification is skipped and any OTP code is accepted
for `+1234567890`.

## Logs and debugging

```bash
make logs-server    # Rust server logs (live-tail)
make logs-db        # PostgreSQL logs
make logs           # All services

# Or directly:
docker compose logs -f server
docker compose logs -f postgres
```

## Common issues

### Tests fail with "no such table" / migration errors

The test harness runs migrations on a fresh per-test database. If a
migration file is malformed or depends on seed data, tests will fail
wholesale. Verify migrations apply to an empty DB:

```bash
make db-reset   # drop, migrate, seed
```

### Server logs show "Compiling…" but no "Listening on"

The Rust server rebuilds on save via `cargo-watch`. If it's stuck
compiling, the most recent edit likely has a compile error. Check:

```bash
docker compose logs --tail=50 server
```

### Server returns 500 with no useful error

Bump log level:

```bash
RUST_LOG=debug docker compose up server
```

or one-shot for a single run:

```bash
docker compose exec server env RUST_LOG=trace cargo run --bin server
```

### GraphQL error but direct HTTP call works

GraphQL resolvers live in `packages/shared/graphql/resolvers/`. A
resolver that doesn't forward all args, mis-renames a field, or
swallows an error is a common cause. Check the specific resolver for
the field that's failing.
