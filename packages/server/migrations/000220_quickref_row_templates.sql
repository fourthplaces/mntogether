-- Row templates that use the quick-ref post template, which was previously
-- dormant because no row template referenced it. quick-ref renders as a
-- compact resource card (QuickRef.tsx) — lighter than directory-ref.

-- ─── pair-quick-ref ───────────────────────────────────────────────────
-- 2 light quick-ref slots. Two compact reference cards side-by-side.
INSERT INTO row_template_configs (slug, display_name, description, layout_variant, sort_order)
VALUES (
  'pair-quick-ref',
  'Quick Reference Pair',
  'Two light quick-reference cards side-by-side. Compact resource listings.',
  'pair',
  310
)
ON CONFLICT (slug) DO NOTHING;

INSERT INTO row_template_slots (row_template_config_id, slot_index, weight, count, post_template_slug)
SELECT id, 0, 'light', 1, 'quick-ref' FROM row_template_configs WHERE slug = 'pair-quick-ref'
ON CONFLICT DO NOTHING;
INSERT INTO row_template_slots (row_template_config_id, slot_index, weight, count, post_template_slug)
SELECT id, 1, 'light', 1, 'quick-ref' FROM row_template_configs WHERE slug = 'pair-quick-ref'
ON CONFLICT DO NOTHING;


-- ─── trio-mixed-ref ───────────────────────────────────────────────────
-- 1 light quick-ref + 1 light digest + 1 light ledger. Mixes a compact
-- reference with other light content for visual variety. More flexible
-- than pair-quick-ref since it only needs 1 light reference post.
INSERT INTO row_template_configs (slug, display_name, description, layout_variant, sort_order)
VALUES (
  'trio-mixed-ref',
  'Mixed Reference Trio',
  'One quick-ref alongside a digest and ledger. Flexible light-weight trio mixing reference with other content.',
  'trio',
  311
)
ON CONFLICT (slug) DO NOTHING;

INSERT INTO row_template_slots (row_template_config_id, slot_index, weight, count, post_template_slug)
SELECT id, 0, 'light', 1, 'quick-ref' FROM row_template_configs WHERE slug = 'trio-mixed-ref'
ON CONFLICT DO NOTHING;
INSERT INTO row_template_slots (row_template_config_id, slot_index, weight, count, post_template_slug)
SELECT id, 1, 'light', 1, 'digest' FROM row_template_configs WHERE slug = 'trio-mixed-ref'
ON CONFLICT DO NOTHING;
INSERT INTO row_template_slots (row_template_config_id, slot_index, weight, count, post_template_slug)
SELECT id, 2, 'light', 1, 'ledger' FROM row_template_configs WHERE slug = 'trio-mixed-ref'
ON CONFLICT DO NOTHING;
