-- Per-county editorial weight target for broadsheet generation.
--
-- Weight is defined as: heavy=3, medium=2, light=1 per post.
-- Root Signal uses this as its target production goal for each county.
-- The layout engine uses it to size the broadsheet, flexing up or down
-- based on the actual post pool's total weight.
--
-- Default 66 ≈ 40 posts of typical mix (6 heavy + 14 medium + 20 light).

ALTER TABLE counties
    ADD COLUMN target_content_weight integer NOT NULL DEFAULT 66
        CHECK (target_content_weight > 0);

COMMENT ON COLUMN counties.target_content_weight IS
    'Editorial weight target for this county''s weekly broadsheet. '
    'Sum of post weights (heavy=3, medium=2, light=1). Root Signal aims '
    'for this total; the layout engine flexes ±30% based on actual pool.';
