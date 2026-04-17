-- Add sort_order to edition_slots for within-cell ordering.
--
-- Motivation: when multiple posts share a slot_index (e.g. classifieds with 2
-- posts per cell, or pair-stack's stacked side), their visual order needs to
-- be explicit and persistable — not derived from created_at — so editors can
-- drag-and-drop to reorder within a cell and have it stick.
--
-- Mirrors the existing pattern on `edition_rows.sort_order`.

ALTER TABLE edition_slots
  ADD COLUMN sort_order INTEGER NOT NULL DEFAULT 0;

-- Backfill: within each (edition_row_id, slot_index) group, assign sort_order
-- in the current visual order (by created_at, then id for stability).
WITH ranked AS (
  SELECT
    id,
    ROW_NUMBER() OVER (
      PARTITION BY edition_row_id, slot_index
      ORDER BY created_at ASC, id ASC
    ) - 1 AS new_order
  FROM edition_slots
)
UPDATE edition_slots es
SET sort_order = ranked.new_order::int
FROM ranked
WHERE es.id = ranked.id;

-- Index for efficient ORDER BY slot_index, sort_order queries per row.
CREATE INDEX idx_edition_slots_row_order
  ON edition_slots (edition_row_id, slot_index, sort_order);
