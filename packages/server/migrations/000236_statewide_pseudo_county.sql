-- Statewide pseudo-county — a virtual "county" whose weekly edition is
-- composed of posts + widgets tagged `service_area='statewide'` only,
-- never county-specific content. Drives the public home page's default
-- view for out-of-state (or pre-geolocation) visitors, and gives editors
-- a first-class place to curate statewide content without duplicating
-- it into 87 individual county editions.
--
-- Design:
--   * `is_pseudo BOOLEAN` flag on counties distinguishes synthetic rows
--     from the 87 real MN counties. All existing filters that implicitly
--     count real counties (dashboards, stats, batch_generate ranges)
--     should treat pseudo rows as first-class for generation but exclude
--     them from "X of 87" style roll-ups. Dashboard updated in this
--     pass; batch_generate_editions iterates counties indiscriminately
--     which is correct — pseudo counties want editions like any other.
--
--   * The layout engine's `load_county_posts` branches on is_pseudo: for
--     pseudo rows it pulls only `service_area='statewide'` posts plus
--     truly-ambient posts (no service_area at all). Real counties still
--     see statewide posts interleaved with county-specific content.
--
--   * fips_code='statewide' is a synthetic identifier distinct from all
--     real county FIPS (5-digit numerics like '27053'). Kept lowercase
--     for parity with the existing `statewide` service_area tag value.

ALTER TABLE counties
    ADD COLUMN is_pseudo BOOLEAN NOT NULL DEFAULT false;

-- Partial index because 99%+ of rows are non-pseudo; this keeps the
-- index tiny and useful for "enumerate pseudo counties" queries.
CREATE INDEX idx_counties_is_pseudo ON counties (is_pseudo) WHERE is_pseudo = true;

-- Insert the Statewide pseudo row. Geographic coordinates point at the
-- approximate center of Minnesota for any future map widgets; they're
-- real values because the column is NOT NULL.
INSERT INTO counties (
    fips_code,
    name,
    state,
    latitude,
    longitude,
    target_content_weight,
    is_pseudo
)
VALUES (
    'statewide',
    'Statewide',
    'MN',
    46.7296,
    -94.6859,
    66,
    true
)
ON CONFLICT (fips_code) DO NOTHING;
