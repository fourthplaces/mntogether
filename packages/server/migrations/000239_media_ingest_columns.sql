-- Media ingest pipeline: columns used by the Root Signal media ingest
-- (server-side fetch + normalise + content-hash dedup of `source_image_url`).
--
-- Spec: docs/handoff-root-signal/ROOT_SIGNAL_API_REQUEST.md §9,
--       docs/guides/ROOT_SIGNAL_MEDIA_INGEST.md.
--
--   source_url         — the original URL we fetched from (provenance).
--   source_ingested_at — when we pulled it (for freshness / audit).
--   content_hash       — SHA-256 of the *normalised* bytes we wrote to
--                        MinIO. Exact-match dedup: a second submission of
--                        an already-ingested image reuses the existing
--                        media row instead of re-uploading.
--
-- The partial unique index enforces exact-match dedup only for rows
-- produced by the ingest path. Pre-existing media rows (content_hash
-- NULL) are unaffected; editor-uploaded images continue not to participate
-- in ingest dedup unless/until a later backfill fills the column.

ALTER TABLE media
  ADD COLUMN source_url TEXT,
  ADD COLUMN source_ingested_at TIMESTAMPTZ,
  ADD COLUMN content_hash TEXT;

CREATE UNIQUE INDEX idx_media_content_hash_unique
  ON media (content_hash)
  WHERE content_hash IS NOT NULL;
