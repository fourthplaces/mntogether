-- Extend the `is_seed` flag pattern (established for posts in migration
-- 000226) to `organizations` and `widgets`. Every top-level entity that
-- the dev seeder inserts now carries is_seed=true so:
--   1. The seed script can wipe its own rows idempotently.
--   2. The admin CMS can visibly label every dummy entity.
--   3. The publish-ready gate can detect seed contamination in editions.
--
-- Default false — real rows stay untouched. Dev backfill at the bottom
-- marks the existing seeded rows so re-seed wipes them cleanly.

ALTER TABLE organizations
    ADD COLUMN is_seed boolean NOT NULL DEFAULT false;

COMMENT ON COLUMN organizations.is_seed IS
    'True when this organization was inserted by data/seed.mjs. Used to '
    'flag dummy entities in the admin CMS and gate edition publish.';

ALTER TABLE widgets
    ADD COLUMN is_seed boolean NOT NULL DEFAULT false;

COMMENT ON COLUMN widgets.is_seed IS
    'True when this widget was inserted by data/seed.mjs. Used to flag '
    'dummy entities in the admin CMS and gate edition publish.';

-- Dev backfill. In a production DB these rows wouldn't exist; in dev
-- every organization and widget in the current DB came from the seed
-- script, so mark them all so `make db-seed` re-runs cleanly. Scoped
-- by origin we can be confident about: `authoring_mode = 'human'` is
-- the widget flavour the seeder uses. Orgs have no equivalent marker,
-- so we mark every existing org — the field only exists post-pivot
-- and the pivot scrub already ran.
UPDATE widgets SET is_seed = true WHERE authoring_mode = 'human';
UPDATE organizations SET is_seed = true;
