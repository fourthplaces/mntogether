-- Bump body_target/body_max on the `feature` post template to reflect the
-- 2-column rendering in `.feat-story__body`. With 2 columns at the 2/3-width
-- cell, a ~400-char body looks sparse (~200 chars per column). Raising the
-- target to 800 chars (~400/column) produces a proper 2-column feature
-- story with visual gravitas. body_max is the soft ceiling before the renderer
-- clamps/truncates.
--
-- This also serves as the minimum content length Root Signal should produce
-- for any `heavy` weight post destined for the feature template.

UPDATE post_template_configs
SET body_target = 800, body_max = 1400
WHERE slug = 'feature';

-- feature-reversed is also heavy but renders in a narrower cell (1/3) without
-- column flow. Bump proportionally but less aggressively.
UPDATE post_template_configs
SET body_target = 400, body_max = 700
WHERE slug = 'feature-reversed';
