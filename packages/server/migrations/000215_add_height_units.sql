-- Add height_units to post_template_configs for layout engine height balancing.
-- Height units are an integer estimate of how tall a post template renders,
-- used to balance stacked cells against their lead cell.

ALTER TABLE post_template_configs ADD COLUMN height_units integer NOT NULL DEFAULT 4;

-- Heavy templates (span-4 / span-6)
UPDATE post_template_configs SET height_units = 12 WHERE slug = 'feature';
UPDATE post_template_configs SET height_units = 10 WHERE slug = 'feature-reversed';

-- Medium templates (span-2 / span-3)
UPDATE post_template_configs SET height_units = 6 WHERE slug = 'gazette';
UPDATE post_template_configs SET height_units = 5 WHERE slug = 'bulletin';
UPDATE post_template_configs SET height_units = 5 WHERE slug = 'alert-notice';
UPDATE post_template_configs SET height_units = 4 WHERE slug = 'card-event';
UPDATE post_template_configs SET height_units = 4 WHERE slug = 'pinboard-exchange';
UPDATE post_template_configs SET height_units = 5 WHERE slug = 'generous-exchange';
UPDATE post_template_configs SET height_units = 5 WHERE slug = 'spotlight-local';
UPDATE post_template_configs SET height_units = 5 WHERE slug = 'directory-ref';

-- Light templates (stacked / compact)
UPDATE post_template_configs SET height_units = 3 WHERE slug = 'ledger';
UPDATE post_template_configs SET height_units = 2 WHERE slug = 'digest';
UPDATE post_template_configs SET height_units = 1 WHERE slug = 'ticker';
UPDATE post_template_configs SET height_units = 3 WHERE slug = 'whisper-notice';
UPDATE post_template_configs SET height_units = 3 WHERE slug = 'quick-ref';
UPDATE post_template_configs SET height_units = 1 WHERE slug = 'ticker-update';

-- Fix pair-stack-gazette: 1 gazette vs 4 gazettes is unbalanced.
-- Change to 1 gazette (anchor) + 3 ledgers (stacked light items).
UPDATE row_template_slots SET weight = 'light', count = 3, post_template_slug = 'ledger'
  WHERE row_template_config_id = (SELECT id FROM row_template_configs WHERE slug = 'pair-stack-gazette')
    AND slot_index = 1;

-- Tickers are full-width only (span-6). Fix row templates that misuse them:

-- classifieds-ticker used tickers in trio (2+2+2) — replace with ledgers
UPDATE row_template_slots SET post_template_slug = 'ledger'
  WHERE row_template_config_id = (SELECT id FROM row_template_configs WHERE slug = 'classifieds-ticker')
    AND post_template_slug = 'ticker';
UPDATE row_template_configs SET slug = 'classifieds-ledger-alt', display_name = 'Classifieds (Ledger Alt)'
  WHERE slug = 'classifieds-ticker';

-- hero-feature-ticker used tickers in lead-stack sidebar (span-2) — replace with digests
UPDATE row_template_slots SET post_template_slug = 'digest'
  WHERE row_template_config_id = (SELECT id FROM row_template_configs WHERE slug = 'hero-feature-ticker')
    AND post_template_slug = 'ticker';
UPDATE row_template_configs SET slug = 'hero-feature-digest', display_name = 'Hero Feature + Digest Sidebar'
  WHERE slug = 'hero-feature-ticker';
