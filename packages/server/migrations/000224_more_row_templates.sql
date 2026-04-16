-- Expand row template variety to reduce unplaced-post counts, especially
-- for small-county pools. Each addition closes a specific gap observed
-- via the `post not placed` diagnostic logs.

-- ═══════════════════════════════════════════════════════════════════════
-- 1. MEDIUM ACTION COVERAGE
--    Problem: alert-notice is the only medium template accepting `action`,
--    and lead-alert-digest was the only row template using it. Medium
--    action posts (e.g. "Register to Vote") got stranded once that row
--    was placed. Add two more homes.
-- ═══════════════════════════════════════════════════════════════════════

-- pair-alert-notice: 2 medium alert-notice side-by-side. Secondary home
-- for action/update medium when lead-alert-digest is used.
INSERT INTO row_template_configs (slug, display_name, description, layout_variant, sort_order)
VALUES ('pair-alert-notice', 'Alert Notice Pair',
  'Two medium alert-notice posts side-by-side. Action or urgent update items.',
  'pair', 320)
ON CONFLICT (slug) DO NOTHING;
INSERT INTO row_template_slots (row_template_config_id, slot_index, weight, count, post_template_slug)
SELECT id, 0, 'medium', 1, 'alert-notice' FROM row_template_configs WHERE slug='pair-alert-notice'
ON CONFLICT DO NOTHING;
INSERT INTO row_template_slots (row_template_config_id, slot_index, weight, count, post_template_slug)
SELECT id, 1, 'medium', 1, 'alert-notice' FROM row_template_configs WHERE slug='pair-alert-notice'
ON CONFLICT DO NOTHING;

-- trio-alert-digest: variant of lead-alert-digest for layout diversity.
INSERT INTO row_template_configs (slug, display_name, description, layout_variant, sort_order)
VALUES ('pair-stack-alert', 'Alert + Digest Stack',
  'One medium alert-notice with three light digests stacked alongside.',
  'pair-stack', 321)
ON CONFLICT (slug) DO NOTHING;
INSERT INTO row_template_slots (row_template_config_id, slot_index, weight, count, post_template_slug)
SELECT id, 0, 'medium', 1, 'alert-notice' FROM row_template_configs WHERE slug='pair-stack-alert'
ON CONFLICT DO NOTHING;
INSERT INTO row_template_slots (row_template_config_id, slot_index, weight, count, post_template_slug)
SELECT id, 1, 'light', 3, 'digest' FROM row_template_configs WHERE slug='pair-stack-alert'
ON CONFLICT DO NOTHING;


-- ═══════════════════════════════════════════════════════════════════════
-- 2. MEDIUM SPOTLIGHT & REFERENCE STACKS
--    spotlight-local and directory-ref only appear in 1+1 pair templates.
--    Add pair-stack variants so they can anchor a row when the pool has
--    surplus lights that need placing.
-- ═══════════════════════════════════════════════════════════════════════

-- pair-stack-spotlight: 1 medium spotlight + 3 light digest
INSERT INTO row_template_configs (slug, display_name, description, layout_variant, sort_order)
VALUES ('pair-stack-spotlight', 'Spotlight + Digest Stack',
  'One medium local spotlight (person or business) with three light digests alongside.',
  'pair-stack', 322)
ON CONFLICT (slug) DO NOTHING;
INSERT INTO row_template_slots (row_template_config_id, slot_index, weight, count, post_template_slug)
SELECT id, 0, 'medium', 1, 'spotlight-local' FROM row_template_configs WHERE slug='pair-stack-spotlight'
ON CONFLICT DO NOTHING;
INSERT INTO row_template_slots (row_template_config_id, slot_index, weight, count, post_template_slug)
SELECT id, 1, 'light', 3, 'digest' FROM row_template_configs WHERE slug='pair-stack-spotlight'
ON CONFLICT DO NOTHING;

-- pair-stack-directory: 1 medium directory-ref + 3 light ledger
INSERT INTO row_template_configs (slug, display_name, description, layout_variant, sort_order)
VALUES ('pair-stack-directory', 'Directory + Ledger Stack',
  'One medium directory reference with three light ledger items stacked.',
  'pair-stack', 323)
ON CONFLICT (slug) DO NOTHING;
INSERT INTO row_template_slots (row_template_config_id, slot_index, weight, count, post_template_slug)
SELECT id, 0, 'medium', 1, 'directory-ref' FROM row_template_configs WHERE slug='pair-stack-directory'
ON CONFLICT DO NOTHING;
INSERT INTO row_template_slots (row_template_config_id, slot_index, weight, count, post_template_slug)
SELECT id, 1, 'light', 3, 'ledger' FROM row_template_configs WHERE slug='pair-stack-directory'
ON CONFLICT DO NOTHING;


-- ═══════════════════════════════════════════════════════════════════════
-- 3. MORE MEDIUM+MEDIUM PAIRS (same-weight, diverse types)
--    Same-variant block limits pair re-use. More pair variety = more
--    rows usable in a single broadsheet.
-- ═══════════════════════════════════════════════════════════════════════

-- pair-gazette-bulletin: 1 medium gazette + 1 medium bulletin
INSERT INTO row_template_configs (slug, display_name, description, layout_variant, sort_order)
VALUES ('pair-gazette-bulletin', 'Gazette + Bulletin Pair',
  'One gazette-style medium alongside a bulletin-style medium. Mixed type pair.',
  'pair', 324)
ON CONFLICT (slug) DO NOTHING;
INSERT INTO row_template_slots (row_template_config_id, slot_index, weight, count, post_template_slug)
SELECT id, 0, 'medium', 1, 'gazette' FROM row_template_configs WHERE slug='pair-gazette-bulletin'
ON CONFLICT DO NOTHING;
INSERT INTO row_template_slots (row_template_config_id, slot_index, weight, count, post_template_slug)
SELECT id, 1, 'medium', 1, 'bulletin' FROM row_template_configs WHERE slug='pair-gazette-bulletin'
ON CONFLICT DO NOTHING;

-- pair-spotlight-bulletin: mixed pair for variety
INSERT INTO row_template_configs (slug, display_name, description, layout_variant, sort_order)
VALUES ('pair-spotlight-bulletin', 'Spotlight + Bulletin Pair',
  'One local spotlight alongside a bulletin-style medium. Mixed type pair.',
  'pair', 325)
ON CONFLICT (slug) DO NOTHING;
INSERT INTO row_template_slots (row_template_config_id, slot_index, weight, count, post_template_slug)
SELECT id, 0, 'medium', 1, 'spotlight-local' FROM row_template_configs WHERE slug='pair-spotlight-bulletin'
ON CONFLICT DO NOTHING;
INSERT INTO row_template_slots (row_template_config_id, slot_index, weight, count, post_template_slug)
SELECT id, 1, 'medium', 1, 'bulletin' FROM row_template_configs WHERE slug='pair-spotlight-bulletin'
ON CONFLICT DO NOTHING;


-- ═══════════════════════════════════════════════════════════════════════
-- 4. LIGHT-ONLY TRIO VARIETY (Phase 3)
--    The dense phase needs more options. trio variants avoid the pair
--    same-variant block when pair has already been used.
-- ═══════════════════════════════════════════════════════════════════════

-- trio-ledger-mix: 1 light ledger + 1 light digest + 1 light ledger
INSERT INTO row_template_configs (slug, display_name, description, layout_variant, sort_order)
VALUES ('trio-ledger-mix', 'Ledger Trio Mix',
  'Three light ledger-style items. Variety of trio-digest for update/reference content.',
  'trio', 326)
ON CONFLICT (slug) DO NOTHING;
INSERT INTO row_template_slots (row_template_config_id, slot_index, weight, count, post_template_slug)
SELECT id, 0, 'light', 1, 'ledger' FROM row_template_configs WHERE slug='trio-ledger-mix'
ON CONFLICT DO NOTHING;
INSERT INTO row_template_slots (row_template_config_id, slot_index, weight, count, post_template_slug)
SELECT id, 1, 'light', 1, 'ledger' FROM row_template_configs WHERE slug='trio-ledger-mix'
ON CONFLICT DO NOTHING;
INSERT INTO row_template_slots (row_template_config_id, slot_index, weight, count, post_template_slug)
SELECT id, 2, 'light', 1, 'ledger' FROM row_template_configs WHERE slug='trio-ledger-mix'
ON CONFLICT DO NOTHING;

-- trio-whisper-digest: 2 light whisper-notice + 1 light digest.
-- Mixes light updates (whisper) with other types.
INSERT INTO row_template_configs (slug, display_name, description, layout_variant, sort_order)
VALUES ('trio-whisper-mix', 'Whisper + Digest Trio',
  'Two light whisper-notices with a digest. Mixed update and story light content.',
  'trio', 327)
ON CONFLICT (slug) DO NOTHING;
INSERT INTO row_template_slots (row_template_config_id, slot_index, weight, count, post_template_slug)
SELECT id, 0, 'light', 1, 'whisper-notice' FROM row_template_configs WHERE slug='trio-whisper-mix'
ON CONFLICT DO NOTHING;
INSERT INTO row_template_slots (row_template_config_id, slot_index, weight, count, post_template_slug)
SELECT id, 1, 'light', 1, 'whisper-notice' FROM row_template_configs WHERE slug='trio-whisper-mix'
ON CONFLICT DO NOTHING;
INSERT INTO row_template_slots (row_template_config_id, slot_index, weight, count, post_template_slug)
SELECT id, 2, 'light', 1, 'digest' FROM row_template_configs WHERE slug='trio-whisper-mix'
ON CONFLICT DO NOTHING;

-- trio-ref-trio: 2 light quick-ref + 1 light ledger
INSERT INTO row_template_configs (slug, display_name, description, layout_variant, sort_order)
VALUES ('trio-ref-ledger', 'Reference + Ledger Trio',
  'Two light quick-reference cards with a ledger. For reference-heavy pools.',
  'trio', 328)
ON CONFLICT (slug) DO NOTHING;
INSERT INTO row_template_slots (row_template_config_id, slot_index, weight, count, post_template_slug)
SELECT id, 0, 'light', 1, 'quick-ref' FROM row_template_configs WHERE slug='trio-ref-ledger'
ON CONFLICT DO NOTHING;
INSERT INTO row_template_slots (row_template_config_id, slot_index, weight, count, post_template_slug)
SELECT id, 1, 'light', 1, 'quick-ref' FROM row_template_configs WHERE slug='trio-ref-ledger'
ON CONFLICT DO NOTHING;
INSERT INTO row_template_slots (row_template_config_id, slot_index, weight, count, post_template_slug)
SELECT id, 2, 'light', 1, 'ledger' FROM row_template_configs WHERE slug='trio-ref-ledger'
ON CONFLICT DO NOTHING;
