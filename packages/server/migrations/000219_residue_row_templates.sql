-- Additional row templates that fill common "residue" patterns —
-- small, odd-shaped post combinations that the current template pool
-- can't absorb. Each template closes a specific gap observed in small
-- counties (e.g. Aitkin) where 1-3 posts get stranded because no row
-- template accepts them.

-- ─── pair-digest ──────────────────────────────────────────────────────
-- 2 light digest slots. Absorbs pairs of light story/update/action/need/aid
-- posts when neither trio-digest (needs 3) nor classifieds (needs 6) fit.
INSERT INTO row_template_configs (slug, display_name, description, layout_variant, sort_order)
VALUES (
  'pair-digest',
  'Digest Pair',
  'Two light digest posts side-by-side. Residue-filler for small pools that cannot feed a full trio.',
  'pair',
  300
)
ON CONFLICT (slug) DO NOTHING;

INSERT INTO row_template_slots (row_template_config_id, slot_index, weight, count, post_template_slug)
SELECT id, 0, 'light', 1, 'digest' FROM row_template_configs WHERE slug = 'pair-digest'
ON CONFLICT DO NOTHING;
INSERT INTO row_template_slots (row_template_config_id, slot_index, weight, count, post_template_slug)
SELECT id, 1, 'light', 1, 'digest' FROM row_template_configs WHERE slug = 'pair-digest'
ON CONFLICT DO NOTHING;


-- ─── pair-bulletin-digest ─────────────────────────────────────────────
-- 1 medium bulletin + 1 light digest. Absorbs (medium reference/business
-- + light story/update) residue that pair-bulletin-ledger can't take
-- because its ledger slot doesn't accept `story`.
INSERT INTO row_template_configs (slug, display_name, description, layout_variant, sort_order)
VALUES (
  'pair-bulletin-digest',
  'Bulletin + Digest Pair',
  'One medium bulletin alongside a light digest. Mixes references/business medium with story/update light.',
  'pair',
  301
)
ON CONFLICT (slug) DO NOTHING;

INSERT INTO row_template_slots (row_template_config_id, slot_index, weight, count, post_template_slug)
SELECT id, 0, 'medium', 1, 'bulletin' FROM row_template_configs WHERE slug = 'pair-bulletin-digest'
ON CONFLICT DO NOTHING;
INSERT INTO row_template_slots (row_template_config_id, slot_index, weight, count, post_template_slug)
SELECT id, 1, 'light', 1, 'digest' FROM row_template_configs WHERE slug = 'pair-bulletin-digest'
ON CONFLICT DO NOTHING;


-- ─── pair-ledger-digest ───────────────────────────────────────────────
-- 1 light ledger + 1 light digest. Pairs a reference/update/event light
-- (ledger) with a story/update/action light (digest) — handy when the
-- pool has mixed light types that don't fit homogeneous pair-ledger.
INSERT INTO row_template_configs (slug, display_name, description, layout_variant, sort_order)
VALUES (
  'pair-ledger-digest',
  'Ledger + Digest Pair',
  'One light ledger post alongside a light digest. Mixes reference/update light with story light.',
  'pair',
  302
)
ON CONFLICT (slug) DO NOTHING;

INSERT INTO row_template_slots (row_template_config_id, slot_index, weight, count, post_template_slug)
SELECT id, 0, 'light', 1, 'ledger' FROM row_template_configs WHERE slug = 'pair-ledger-digest'
ON CONFLICT DO NOTHING;
INSERT INTO row_template_slots (row_template_config_id, slot_index, weight, count, post_template_slug)
SELECT id, 1, 'light', 1, 'digest' FROM row_template_configs WHERE slug = 'pair-ledger-digest'
ON CONFLICT DO NOTHING;


-- ─── trio-gazette-digest ──────────────────────────────────────────────
-- 1 medium gazette + 2 light digest. Fills (medium + 2 light) residue with
-- the broadest type compatibility — gazette accepts every post_type,
-- digest accepts story/update/action/need/aid. Competes with
-- lead-alert-digest (alert-notice only accepts update/action) for pools
-- with heavier story/reference content.
INSERT INTO row_template_configs (slug, display_name, description, layout_variant, sort_order)
VALUES (
  'trio-gazette-digest',
  'Gazette + Digest Trio',
  'One medium gazette with two light digest stacked alongside. Flexible 1m+2l fill for mixed pools.',
  'lead-stack',
  303
)
ON CONFLICT (slug) DO NOTHING;

INSERT INTO row_template_slots (row_template_config_id, slot_index, weight, count, post_template_slug)
SELECT id, 0, 'medium', 1, 'gazette' FROM row_template_configs WHERE slug = 'trio-gazette-digest'
ON CONFLICT DO NOTHING;
INSERT INTO row_template_slots (row_template_config_id, slot_index, weight, count, post_template_slug)
SELECT id, 1, 'light', 2, 'digest' FROM row_template_configs WHERE slug = 'trio-gazette-digest'
ON CONFLICT DO NOTHING;
