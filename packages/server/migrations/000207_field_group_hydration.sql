-- Field Group Hydration: add missing field group tables + deck column
--
-- Adds 3 tables that exist in the prototype spec (POST-DATA-MODEL.md)
-- but were not created in the original Phase 2 migration (000173).
-- Also adds the `deck` column to post_meta (subtitle/subhead, 60-150 chars).

-- ============================================================================
-- 1. post_datetime — event timing (1:1 with post)
-- ============================================================================

CREATE TABLE IF NOT EXISTS post_datetime (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    post_id     UUID NOT NULL UNIQUE REFERENCES posts(id) ON DELETE CASCADE,
    start_at    TIMESTAMPTZ,
    end_at      TIMESTAMPTZ,
    cost        TEXT,
    recurring   BOOLEAN NOT NULL DEFAULT false
);
CREATE INDEX IF NOT EXISTS idx_post_datetime_post_id ON post_datetime(post_id);

-- ============================================================================
-- 2. post_status — exchange state tracking (1:1 with post)
-- ============================================================================

CREATE TABLE IF NOT EXISTS post_status (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    post_id     UUID NOT NULL UNIQUE REFERENCES posts(id) ON DELETE CASCADE,
    state       TEXT,
    verified    TEXT
);
CREATE INDEX IF NOT EXISTS idx_post_status_post_id ON post_status(post_id);

-- ============================================================================
-- 3. post_schedule — hours/schedule entries (1:many with post)
-- ============================================================================

CREATE TABLE IF NOT EXISTS post_schedule (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    post_id     UUID NOT NULL REFERENCES posts(id) ON DELETE CASCADE,
    day         TEXT NOT NULL,
    opens       TEXT NOT NULL,
    closes      TEXT NOT NULL,
    sort_order  INT NOT NULL DEFAULT 0,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS idx_post_schedule_post_id ON post_schedule(post_id);

-- ============================================================================
-- 4. Add deck column to post_meta (subtitle/subhead)
-- ============================================================================

ALTER TABLE post_meta ADD COLUMN IF NOT EXISTS deck TEXT;
