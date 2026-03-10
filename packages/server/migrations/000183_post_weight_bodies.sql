-- =============================================================================
-- Weight-specific body text columns for Root Signal integration
-- =============================================================================
-- Root Signal provides pre-written body text at 3 weight tiers per post:
--   body_heavy  — longer text for feature-weight templates (~500 chars)
--   body_medium — moderate text for gazette/bulletin templates (~200 chars)
--   body_light  — short text for digest/ticker templates (~100 chars)
--
-- All nullable. Root Signal populates all 3; manual posts continue using
-- `description`. The frontend selects the weight-appropriate body text
-- based on the assigned template's weight tier, falling back to
-- `description` + truncation if no weight-specific body exists.
-- =============================================================================

ALTER TABLE posts ADD COLUMN body_heavy TEXT;
ALTER TABLE posts ADD COLUMN body_medium TEXT;
ALTER TABLE posts ADD COLUMN body_light TEXT;
