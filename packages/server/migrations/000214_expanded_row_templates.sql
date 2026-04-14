-- =============================================================================
-- Expanded row templates from design prototype RT catalog
-- =============================================================================
-- Adds 14 new row template recipes covering lead, lead-stack, pair,
-- pair-stack, and trio layout variants not yet in the database.
-- All inserts are idempotent via ON CONFLICT (slug) DO NOTHING.
-- =============================================================================

-- ---------------------------------------------------------------------------
-- LEAD variant
-- ---------------------------------------------------------------------------

-- RT-05: lead-feature-event — heavy feature-reversed + medium card-event
INSERT INTO row_template_configs (slug, display_name, description, sort_order, layout_variant) VALUES
('lead-feature-event', 'Feature + Event Card', 'Reversed feature with event card sidebar', 30, 'lead')
ON CONFLICT (slug) DO NOTHING;

INSERT INTO row_template_slots (row_template_config_id, slot_index, weight, count, post_template_slug) VALUES
((SELECT id FROM row_template_configs WHERE slug = 'lead-feature-event'), 0, 'heavy', 1, 'feature-reversed'),
((SELECT id FROM row_template_configs WHERE slug = 'lead-feature-event'), 1, 'medium', 1, 'card-event');

-- ---------------------------------------------------------------------------
-- LEAD-STACK variants
-- ---------------------------------------------------------------------------

-- RT-07: lead-feature-cards — heavy feature + 2x medium card-event stacked
INSERT INTO row_template_configs (slug, display_name, description, sort_order, layout_variant) VALUES
('lead-feature-cards', 'Feature + Event Cards', 'Feature hero with stacked event cards in sidebar', 31, 'lead-stack')
ON CONFLICT (slug) DO NOTHING;

INSERT INTO row_template_slots (row_template_config_id, slot_index, weight, count, post_template_slug) VALUES
((SELECT id FROM row_template_configs WHERE slug = 'lead-feature-cards'), 0, 'heavy', 1, 'feature'),
((SELECT id FROM row_template_configs WHERE slug = 'lead-feature-cards'), 1, 'medium', 2, 'card-event');

-- RT-09: lead-alert-digest — medium alert-notice + 3x light digest stacked
INSERT INTO row_template_configs (slug, display_name, description, sort_order, layout_variant) VALUES
('lead-alert-digest', 'Alert + Digest Stack', 'Alert notice with stacked digest items in sidebar', 32, 'lead-stack')
ON CONFLICT (slug) DO NOTHING;

INSERT INTO row_template_slots (row_template_config_id, slot_index, weight, count, post_template_slug) VALUES
((SELECT id FROM row_template_configs WHERE slug = 'lead-alert-digest'), 0, 'medium', 1, 'alert-notice'),
((SELECT id FROM row_template_configs WHERE slug = 'lead-alert-digest'), 1, 'light', 3, 'digest');

-- ---------------------------------------------------------------------------
-- PAIR variants
-- ---------------------------------------------------------------------------

-- RT-10: pair-spotlight — 2x medium spotlight-local
INSERT INTO row_template_configs (slug, display_name, description, sort_order, layout_variant) VALUES
('pair-spotlight', 'Double Spotlight', 'Two spotlight cards side by side', 40, 'pair')
ON CONFLICT (slug) DO NOTHING;

INSERT INTO row_template_slots (row_template_config_id, slot_index, weight, count, post_template_slug) VALUES
((SELECT id FROM row_template_configs WHERE slug = 'pair-spotlight'), 0, 'medium', 1, 'spotlight-local'),
((SELECT id FROM row_template_configs WHERE slug = 'pair-spotlight'), 1, 'medium', 1, 'spotlight-local');

-- RT-12: pair-directory-spotlight — medium directory-ref + medium spotlight-local
INSERT INTO row_template_configs (slug, display_name, description, sort_order, layout_variant) VALUES
('pair-directory-spotlight', 'Directory + Spotlight', 'Directory reference paired with spotlight card', 41, 'pair')
ON CONFLICT (slug) DO NOTHING;

INSERT INTO row_template_slots (row_template_config_id, slot_index, weight, count, post_template_slug) VALUES
((SELECT id FROM row_template_configs WHERE slug = 'pair-directory-spotlight'), 0, 'medium', 1, 'directory-ref'),
((SELECT id FROM row_template_configs WHERE slug = 'pair-directory-spotlight'), 1, 'medium', 1, 'spotlight-local');

-- RT-14: pair-exchange — 2x medium generous-exchange
INSERT INTO row_template_configs (slug, display_name, description, sort_order, layout_variant) VALUES
('pair-exchange', 'Double Exchange', 'Two generous exchange cards side by side', 42, 'pair')
ON CONFLICT (slug) DO NOTHING;

INSERT INTO row_template_slots (row_template_config_id, slot_index, weight, count, post_template_slug) VALUES
((SELECT id FROM row_template_configs WHERE slug = 'pair-exchange'), 0, 'medium', 1, 'generous-exchange'),
((SELECT id FROM row_template_configs WHERE slug = 'pair-exchange'), 1, 'medium', 1, 'generous-exchange');

-- RT-15: pair-ledger — 2x light ledger
INSERT INTO row_template_configs (slug, display_name, description, sort_order, layout_variant) VALUES
('pair-ledger', 'Double Ledger', 'Two compact ledger columns side by side', 43, 'pair')
ON CONFLICT (slug) DO NOTHING;

INSERT INTO row_template_slots (row_template_config_id, slot_index, weight, count, post_template_slug) VALUES
((SELECT id FROM row_template_configs WHERE slug = 'pair-ledger'), 0, 'light', 1, 'ledger'),
((SELECT id FROM row_template_configs WHERE slug = 'pair-ledger'), 1, 'light', 1, 'ledger');

-- RT-16: pair-bulletin-ledger — medium bulletin + light ledger
INSERT INTO row_template_configs (slug, display_name, description, sort_order, layout_variant) VALUES
('pair-bulletin-ledger', 'Bulletin + Ledger', 'Bulletin card with compact ledger sidebar', 44, 'pair')
ON CONFLICT (slug) DO NOTHING;

INSERT INTO row_template_slots (row_template_config_id, slot_index, weight, count, post_template_slug) VALUES
((SELECT id FROM row_template_configs WHERE slug = 'pair-bulletin-ledger'), 0, 'medium', 1, 'bulletin'),
((SELECT id FROM row_template_configs WHERE slug = 'pair-bulletin-ledger'), 1, 'light', 1, 'ledger');

-- RT-17: pair-gazette-spotlight — medium gazette + medium spotlight-local
INSERT INTO row_template_configs (slug, display_name, description, sort_order, layout_variant) VALUES
('pair-gazette-spotlight', 'Gazette + Spotlight', 'Gazette article paired with spotlight card', 45, 'pair')
ON CONFLICT (slug) DO NOTHING;

INSERT INTO row_template_slots (row_template_config_id, slot_index, weight, count, post_template_slug) VALUES
((SELECT id FROM row_template_configs WHERE slug = 'pair-gazette-spotlight'), 0, 'medium', 1, 'gazette'),
((SELECT id FROM row_template_configs WHERE slug = 'pair-gazette-spotlight'), 1, 'medium', 1, 'spotlight-local');

-- ---------------------------------------------------------------------------
-- PAIR-STACK variant
-- ---------------------------------------------------------------------------

-- RT-18: pair-stack-gazette — medium gazette + 4x medium gazette stacked
INSERT INTO row_template_configs (slug, display_name, description, sort_order, layout_variant) VALUES
('pair-stack-gazette', 'Gazette + Gazette Stack', 'Gazette with four stacked gazette items', 50, 'pair-stack')
ON CONFLICT (slug) DO NOTHING;

INSERT INTO row_template_slots (row_template_config_id, slot_index, weight, count, post_template_slug) VALUES
((SELECT id FROM row_template_configs WHERE slug = 'pair-stack-gazette'), 0, 'medium', 1, 'gazette'),
((SELECT id FROM row_template_configs WHERE slug = 'pair-stack-gazette'), 1, 'medium', 4, 'gazette');

-- ---------------------------------------------------------------------------
-- TRIO variants
-- ---------------------------------------------------------------------------

-- RT-20: trio-whisper — 3x light whisper-notice
INSERT INTO row_template_configs (slug, display_name, description, sort_order, layout_variant) VALUES
('trio-whisper', 'Triple Whisper', 'Three whisper notice cards', 60, 'trio')
ON CONFLICT (slug) DO NOTHING;

INSERT INTO row_template_slots (row_template_config_id, slot_index, weight, count, post_template_slug) VALUES
((SELECT id FROM row_template_configs WHERE slug = 'trio-whisper'), 0, 'light', 1, 'whisper-notice'),
((SELECT id FROM row_template_configs WHERE slug = 'trio-whisper'), 1, 'light', 1, 'whisper-notice'),
((SELECT id FROM row_template_configs WHERE slug = 'trio-whisper'), 2, 'light', 1, 'whisper-notice');

-- RT-21: trio-digest — 3x light digest
INSERT INTO row_template_configs (slug, display_name, description, sort_order, layout_variant) VALUES
('trio-digest', 'Triple Digest', 'Three compact digest columns', 61, 'trio')
ON CONFLICT (slug) DO NOTHING;

INSERT INTO row_template_slots (row_template_config_id, slot_index, weight, count, post_template_slug) VALUES
((SELECT id FROM row_template_configs WHERE slug = 'trio-digest'), 0, 'light', 1, 'digest'),
((SELECT id FROM row_template_configs WHERE slug = 'trio-digest'), 1, 'light', 1, 'digest'),
((SELECT id FROM row_template_configs WHERE slug = 'trio-digest'), 2, 'light', 1, 'digest');

-- RT-23: trio-pinboard — 3x medium pinboard-exchange
INSERT INTO row_template_configs (slug, display_name, description, sort_order, layout_variant) VALUES
('trio-pinboard', 'Triple Pinboard', 'Three pinboard exchange cards', 62, 'trio')
ON CONFLICT (slug) DO NOTHING;

INSERT INTO row_template_slots (row_template_config_id, slot_index, weight, count, post_template_slug) VALUES
((SELECT id FROM row_template_configs WHERE slug = 'trio-pinboard'), 0, 'medium', 1, 'pinboard-exchange'),
((SELECT id FROM row_template_configs WHERE slug = 'trio-pinboard'), 1, 'medium', 1, 'pinboard-exchange'),
((SELECT id FROM row_template_configs WHERE slug = 'trio-pinboard'), 2, 'medium', 1, 'pinboard-exchange');

-- RT-24: trio-mixed-gazette — 3x medium gazette
INSERT INTO row_template_configs (slug, display_name, description, sort_order, layout_variant) VALUES
('trio-mixed-gazette', 'Triple Gazette', 'Three gazette cards in a row', 63, 'trio')
ON CONFLICT (slug) DO NOTHING;

INSERT INTO row_template_slots (row_template_config_id, slot_index, weight, count, post_template_slug) VALUES
((SELECT id FROM row_template_configs WHERE slug = 'trio-mixed-gazette'), 0, 'medium', 1, 'gazette'),
((SELECT id FROM row_template_configs WHERE slug = 'trio-mixed-gazette'), 1, 'medium', 1, 'gazette'),
((SELECT id FROM row_template_configs WHERE slug = 'trio-mixed-gazette'), 2, 'medium', 1, 'gazette');

-- Bulletin Events 2×2: pair layout, 2 bul-event per cell (4 events total).
-- Events shown as bulletin cards at span-3 width, 2 stacked per column.
INSERT INTO row_template_configs (slug, display_name, layout_variant, sort_order)
VALUES ('pair-bulletin-event', 'Bulletin Events (2×2)', 'pair', 50)
ON CONFLICT DO NOTHING;
INSERT INTO row_template_slots (row_template_config_id, slot_index, weight, count, post_template_slug, accepts)
SELECT id, 0, 'medium', 2, 'bulletin', ARRAY['event']
FROM row_template_configs WHERE slug = 'pair-bulletin-event';
INSERT INTO row_template_slots (row_template_config_id, slot_index, weight, count, post_template_slug, accepts)
SELECT id, 1, 'medium', 2, 'bulletin', ARRAY['event']
FROM row_template_configs WHERE slug = 'pair-bulletin-event';
