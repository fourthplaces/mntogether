-- =============================================================================
-- Specialty post templates + weight column
-- =============================================================================
-- Adds a `weight` column to post_template_configs so the layout engine can
-- match templates by weight from the DB instead of hardcoding slug→weight maps.
--
-- Registers 9 specialty React components as first-class post templates so the
-- layout engine and row recipes can assign them.
-- =============================================================================

-- 1. Add weight column to post_template_configs
ALTER TABLE post_template_configs ADD COLUMN weight TEXT NOT NULL DEFAULT 'medium';

-- 2. Backfill weights for existing templates
UPDATE post_template_configs SET weight = 'heavy' WHERE slug IN ('feature', 'feature-reversed');
UPDATE post_template_configs SET weight = 'light' WHERE slug IN ('ledger', 'ticker', 'digest');
-- gazette and bulletin remain 'medium' (the default)

-- 3. Insert specialty templates
INSERT INTO post_template_configs (slug, display_name, description, compatible_types, body_target, body_max, title_max, sort_order, weight) VALUES
('alert-notice',       'Alert Notice',        'Urgent notice with alert flag and dramatic styling.',           '{notice}',    180, 240, 60,  8, 'medium'),
('pinboard-exchange',  'Pinboard Exchange',   'Pinboard-style need/offer card with status display.',           '{exchange}',  180, 240, 60,  9, 'medium'),
('card-event',         'Card Event',          'Event card with date circle, when/where details.',              '{event}',     160, 220, 60, 10, 'medium'),
('quick-ref',          'Quick Reference',     'Compact reference with count and items list.',                  '{reference}',   0,   0, 50, 11, 'light'),
('directory-ref',      'Directory Reference', 'Directory listing with items and updated date.',                '{reference}',   0,   0, 60, 12, 'medium'),
('generous-exchange',  'Generous Exchange',   'Rich exchange card with header tag, body, and status.',         '{exchange}',  180, 240, 60, 13, 'medium'),
('whisper-notice',     'Whisper Notice',      'Quiet update style with timestamp, title, and short body.',     '{notice}',    120, 160, 50, 14, 'light'),
('spotlight-local',    'Broadsheet Spotlight', 'Support Local spotlight with name, tagline, and body.',        '{spotlight}', 180, 240, 60, 15, 'medium'),
('ticker-update',      'Ticker Update',       'Ticker-style notice with timestamp, title, and source line.',   '{notice}',      0,   0, 50, 16, 'light')
ON CONFLICT (slug) DO NOTHING;
