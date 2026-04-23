-- Root Signal ingest-endpoint schema.
--
-- Everything the new `POST /Posts/create_post` handler needs:
--
--   * api_keys              — machine-token storage for Root Signal (and any
--                             future service clients). Hash-only, rotateable,
--                             scope-gated.
--   * api_idempotency_keys  — (api_key, X-Idempotency-Key) → stored response.
--                             Lets the handler return the original 201 body on
--                             a safe retry and 409 on payload divergence.
--   * source_individuals    — the parallel to `organizations` for individual
--                             (non-org) source authors. Dedup'd by
--                             (platform, handle) first, then platform_url.
--   * post_sources columns  — Addendum 01 metadata (content_hash, snippet,
--                             confidence, platform_id, platform_post_type_hint)
--                             plus is_primary, so the ingest handler can carry
--                             per-citation data through to the sources panel.
--   * posts.content_hash    — §1.5 dedup key. Normalised title + source_url +
--                             day-bucket(published_at) + sorted service_area
--                             slugs. Matching hash → refresh published_at and
--                             return the existing post_id.
--   * posts status          — add 'in_review' for soft-fail ingests (low
--                             confidence, stale source metadata, missing deck
--                             on heavy, etc.). Editors clear from the Signal
--                             Inbox UI (Worktree 5).
--
-- Design notes:
--
--   * api_keys.token_hash stores SHA-256(plaintext). Plaintext is shown once at
--     issuance and never again. The prefix (`rsk_live_`, `rsk_test_`, `rsk_dev_`)
--     is stored for operator triage — log lines show the prefix but never the
--     token body.
--
--   * api_idempotency_keys.payload_hash is the SHA-256 hex of the canonicalised
--     request body (sorted JSON keys, insignificant whitespace stripped). The
--     handler computes it on both writes and reads to detect payload divergence.
--
--   * source_individuals parallels organizations but does not link through the
--     unified `sources` table — individuals are authors, not scrape targets. A
--     post_sources row for an individual carries source_type = 'individual' and
--     source_id = source_individuals.id directly.
--
--   * The is_primary column on post_sources picks the row that feeds
--     post_source_attribution. Exactly one row per post should be is_primary;
--     enforced by a partial unique index.

-- =============================================================================
-- 1. api_keys
-- =============================================================================

CREATE TABLE api_keys (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    client_name     TEXT NOT NULL,
    prefix          TEXT NOT NULL,
    token_hash      TEXT NOT NULL UNIQUE,
    scopes          TEXT[] NOT NULL DEFAULT ARRAY[]::TEXT[],
    rotated_from_id UUID REFERENCES api_keys(id) ON DELETE SET NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    revoked_at      TIMESTAMPTZ,
    last_used_at    TIMESTAMPTZ
);

CREATE INDEX idx_api_keys_active_token
    ON api_keys(token_hash)
    WHERE revoked_at IS NULL;

CREATE INDEX idx_api_keys_client_name
    ON api_keys(client_name)
    WHERE revoked_at IS NULL;

-- =============================================================================
-- 2. api_idempotency_keys
-- =============================================================================

CREATE TABLE api_idempotency_keys (
    key              UUID PRIMARY KEY,
    api_key_id       UUID NOT NULL REFERENCES api_keys(id) ON DELETE CASCADE,
    payload_hash     TEXT NOT NULL,
    response_status  INT NOT NULL,
    response_body    JSONB NOT NULL,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_api_idempotency_api_key_created
    ON api_idempotency_keys(api_key_id, created_at);

-- =============================================================================
-- 3. source_individuals
-- =============================================================================

CREATE TABLE source_individuals (
    id                    UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    display_name          TEXT NOT NULL,
    handle                TEXT,
    platform              TEXT,
    platform_url          TEXT,
    verified_identity     BOOLEAN NOT NULL DEFAULT false,
    consent_to_publish    BOOLEAN NOT NULL DEFAULT false,
    consent_source        TEXT,
    consent_captured_at   TIMESTAMPTZ,
    created_at            TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at            TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT source_individuals_platform_check
        CHECK (platform IS NULL OR platform IN (
            'instagram', 'twitter', 'tiktok', 'facebook',
            'bluesky', 'youtube', 'substack', 'other'
        ))
);

-- Dedup ladder step 1: (platform, handle). Partial index because handle can
-- legitimately be NULL (individuals cited only by display name / URL).
CREATE UNIQUE INDEX idx_source_individuals_platform_handle
    ON source_individuals(platform, handle)
    WHERE handle IS NOT NULL AND platform IS NOT NULL;

-- Dedup ladder step 2: platform_url.
CREATE INDEX idx_source_individuals_platform_url
    ON source_individuals(platform_url)
    WHERE platform_url IS NOT NULL;

-- =============================================================================
-- 4. post_sources — Addendum 01 citation metadata
-- =============================================================================

ALTER TABLE post_sources
    ADD COLUMN content_hash            TEXT,
    ADD COLUMN snippet                 TEXT,
    ADD COLUMN confidence              INT,
    ADD COLUMN platform_id             TEXT,
    ADD COLUMN platform_post_type_hint TEXT,
    ADD COLUMN is_primary              BOOLEAN NOT NULL DEFAULT false;

-- Exactly one primary source per post.
CREATE UNIQUE INDEX idx_post_sources_one_primary_per_post
    ON post_sources(post_id)
    WHERE is_primary = true;

-- Dedup against prior citations carrying the same hash.
CREATE INDEX idx_post_sources_content_hash
    ON post_sources(content_hash)
    WHERE content_hash IS NOT NULL;

-- Allow 'individual' as a post_sources.source_type so the handler can store
-- individual-sourced citations by pointing source_id → source_individuals.id.
-- There's no existing CHECK constraint on source_type (see migration 000148);
-- this is a documentation hook only.
COMMENT ON COLUMN post_sources.source_type IS
    'website | instagram | facebook | x | tiktok | bluesky | youtube | substack | other | individual';

-- =============================================================================
-- 5. post_media — alt_text (spec §9.1 requires per-entry accessibility text)
-- =============================================================================

ALTER TABLE post_media
    ADD COLUMN alt_text TEXT;

-- =============================================================================
-- 6. posts — content_hash for §1.5 dedup + 'in_review' status
-- =============================================================================

ALTER TABLE posts
    ADD COLUMN content_hash TEXT;

CREATE INDEX idx_posts_content_hash
    ON posts(content_hash)
    WHERE content_hash IS NOT NULL AND deleted_at IS NULL;

-- Extend the status constraint with 'in_review' for soft-failed ingests.
-- Existing valid states: draft, pending_approval, active, filled, rejected,
-- expired, archived (from migrations 000151 + 000168).
ALTER TABLE posts DROP CONSTRAINT IF EXISTS listings_status_check;
ALTER TABLE posts ADD CONSTRAINT listings_status_check
    CHECK (status IN (
        'draft', 'pending_approval', 'in_review',
        'active', 'filled', 'rejected', 'expired', 'archived'
    ));
