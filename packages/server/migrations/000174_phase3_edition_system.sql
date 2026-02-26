-- Phase 3: Edition System
-- Creates the broadsheet/edition data model for county-scoped weekly editions.
-- See docs/CMS_SYSTEM_SPEC.md §7 and docs/ROOT_EDITORIAL_PIVOT.md Phase 3.

-- =============================================================================
-- 1. COUNTIES — Reference table for Minnesota's 87 counties
-- =============================================================================

CREATE TABLE counties (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    fips_code   TEXT UNIQUE NOT NULL,
    name        TEXT NOT NULL,
    state       TEXT NOT NULL DEFAULT 'MN',
    latitude    DOUBLE PRECISION NOT NULL,
    longitude   DOUBLE PRECISION NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_counties_name ON counties(name);

-- =============================================================================
-- 2. ZIP_COUNTIES — Maps zip codes to counties (computed via haversine)
-- =============================================================================

CREATE TABLE zip_counties (
    zip_code    TEXT NOT NULL REFERENCES zip_codes(zip_code),
    county_id   UUID NOT NULL REFERENCES counties(id),
    is_primary  BOOLEAN NOT NULL DEFAULT true,
    PRIMARY KEY (zip_code, county_id)
);

CREATE INDEX idx_zip_counties_county ON zip_counties(county_id);

-- =============================================================================
-- 3. ROW_TEMPLATE_CONFIGS — Available row layouts for broadsheet
-- =============================================================================

CREATE TABLE row_template_configs (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    slug            TEXT UNIQUE NOT NULL,
    display_name    TEXT NOT NULL,
    description     TEXT,
    sort_order      INT NOT NULL DEFAULT 0,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- =============================================================================
-- 4. ROW_TEMPLATE_SLOTS — Normalized slot definitions per row template
-- =============================================================================

CREATE TABLE row_template_slots (
    id                      UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    row_template_config_id  UUID NOT NULL REFERENCES row_template_configs(id) ON DELETE CASCADE,
    slot_index              INT NOT NULL,
    weight                  TEXT NOT NULL,
    count                   INT NOT NULL DEFAULT 1,
    accepts                 TEXT[],
    UNIQUE(row_template_config_id, slot_index)
);

CREATE INDEX idx_row_template_slots_template ON row_template_slots(row_template_config_id);

-- =============================================================================
-- 5. POST_TEMPLATE_CONFIGS — Visual treatment specs + character limits
-- =============================================================================

CREATE TABLE post_template_configs (
    id                UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    slug              TEXT UNIQUE NOT NULL,
    display_name      TEXT NOT NULL,
    description       TEXT,
    compatible_types  TEXT[] NOT NULL,
    body_target       INT NOT NULL,
    body_max          INT NOT NULL,
    title_max         INT NOT NULL,
    sort_order        INT NOT NULL DEFAULT 0,
    created_at        TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- =============================================================================
-- 6. EDITIONS — One edition per county per publication period
-- =============================================================================

CREATE TABLE editions (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    county_id       UUID NOT NULL REFERENCES counties(id),
    title           TEXT,
    period_start    DATE NOT NULL,
    period_end      DATE NOT NULL,
    status          TEXT NOT NULL DEFAULT 'draft',
    published_at    TIMESTAMPTZ,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(county_id, period_start)
);

CREATE INDEX idx_editions_county ON editions(county_id);
CREATE INDEX idx_editions_status ON editions(status);
CREATE INDEX idx_editions_period ON editions(period_start, period_end);

-- =============================================================================
-- 7. EDITION_ROWS — Ordered rows within an edition
-- =============================================================================

CREATE TABLE edition_rows (
    id                      UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    edition_id              UUID NOT NULL REFERENCES editions(id) ON DELETE CASCADE,
    row_template_config_id  UUID NOT NULL REFERENCES row_template_configs(id),
    sort_order              INT NOT NULL DEFAULT 0,
    created_at              TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_edition_rows_edition ON edition_rows(edition_id);

-- =============================================================================
-- 8. EDITION_SLOTS — Posts placed within row slots
-- =============================================================================

CREATE TABLE edition_slots (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    edition_row_id  UUID NOT NULL REFERENCES edition_rows(id) ON DELETE CASCADE,
    post_id         UUID NOT NULL REFERENCES posts(id),
    post_template   TEXT NOT NULL,
    slot_index      INT NOT NULL DEFAULT 0,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_edition_slots_row ON edition_slots(edition_row_id);
CREATE INDEX idx_edition_slots_post ON edition_slots(post_id);

-- =============================================================================
-- SEED DATA: 87 Minnesota Counties (FIPS codes + approximate centroids)
-- =============================================================================

INSERT INTO counties (fips_code, name, state, latitude, longitude) VALUES
('27001', 'Aitkin',             'MN', 46.60, -93.42),
('27003', 'Anoka',              'MN', 45.27, -93.25),
('27005', 'Becker',             'MN', 46.93, -95.67),
('27007', 'Beltrami',           'MN', 47.97, -94.93),
('27009', 'Benton',             'MN', 45.70, -94.00),
('27011', 'Big Stone',          'MN', 45.42, -96.42),
('27013', 'Blue Earth',         'MN', 44.03, -94.07),
('27015', 'Brown',              'MN', 44.24, -94.73),
('27017', 'Carlton',            'MN', 46.66, -92.68),
('27019', 'Carver',             'MN', 44.82, -93.80),
('27021', 'Cass',               'MN', 47.00, -94.32),
('27023', 'Chippewa',           'MN', 44.97, -95.57),
('27025', 'Chisago',            'MN', 45.50, -92.90),
('27027', 'Clay',               'MN', 46.88, -96.50),
('27029', 'Clearwater',         'MN', 47.57, -95.38),
('27031', 'Cook',               'MN', 47.92, -90.53),
('27033', 'Cottonwood',         'MN', 44.00, -95.18),
('27035', 'Crow Wing',          'MN', 46.48, -94.07),
('27037', 'Dakota',             'MN', 44.67, -93.07),
('27039', 'Dodge',              'MN', 44.02, -92.87),
('27041', 'Douglas',            'MN', 45.93, -95.45),
('27043', 'Faribault',          'MN', 43.67, -93.95),
('27045', 'Fillmore',           'MN', 43.67, -92.10),
('27047', 'Freeborn',           'MN', 43.67, -93.35),
('27049', 'Goodhue',            'MN', 44.40, -92.72),
('27051', 'Grant',              'MN', 45.93, -96.02),
('27053', 'Hennepin',           'MN', 45.00, -93.47),
('27055', 'Houston',            'MN', 43.67, -91.50),
('27057', 'Hubbard',            'MN', 47.10, -94.90),
('27059', 'Isanti',             'MN', 45.55, -93.30),
('27061', 'Itasca',             'MN', 47.50, -93.63),
('27063', 'Jackson',            'MN', 43.67, -95.15),
('27065', 'Kanabec',            'MN', 45.95, -93.30),
('27067', 'Kandiyohi',          'MN', 45.15, -95.00),
('27069', 'Kittson',            'MN', 48.77, -96.78),
('27071', 'Koochiching',        'MN', 48.23, -93.77),
('27073', 'Lac qui Parle',      'MN', 44.98, -96.17),
('27075', 'Lake',               'MN', 47.52, -91.40),
('27077', 'Lake of the Woods',  'MN', 48.77, -94.90),
('27079', 'Le Sueur',           'MN', 44.37, -93.73),
('27081', 'Lincoln',            'MN', 44.42, -96.27),
('27083', 'Lyon',               'MN', 44.42, -95.83),
('27085', 'McLeod',             'MN', 44.82, -94.27),
('27087', 'Mahnomen',           'MN', 47.32, -95.82),
('27089', 'Marshall',           'MN', 48.35, -96.37),
('27091', 'Martin',             'MN', 43.67, -94.55),
('27093', 'Meeker',             'MN', 45.12, -94.52),
('27095', 'Mille Lacs',         'MN', 45.93, -93.63),
('27097', 'Morrison',           'MN', 46.02, -94.27),
('27099', 'Mower',              'MN', 43.67, -92.75),
('27101', 'Murray',             'MN', 44.02, -95.77),
('27103', 'Nicollet',           'MN', 44.35, -94.25),
('27105', 'Nobles',             'MN', 43.67, -95.75),
('27107', 'Norman',             'MN', 47.33, -96.45),
('27109', 'Olmsted',            'MN', 44.00, -92.40),
('27111', 'Otter Tail',         'MN', 46.40, -95.72),
('27113', 'Pennington',         'MN', 48.07, -96.05),
('27115', 'Pine',               'MN', 46.12, -92.75),
('27117', 'Pipestone',          'MN', 44.02, -96.32),
('27119', 'Polk',               'MN', 47.77, -96.40),
('27121', 'Pope',               'MN', 45.58, -95.45),
('27123', 'Ramsey',             'MN', 45.02, -93.10),
('27125', 'Red Lake',           'MN', 47.87, -96.10),
('27127', 'Redwood',            'MN', 44.40, -95.25),
('27129', 'Renville',           'MN', 44.72, -94.95),
('27131', 'Rice',               'MN', 44.35, -93.30),
('27133', 'Rock',               'MN', 43.67, -96.25),
('27135', 'Roseau',             'MN', 48.78, -95.78),
('27137', 'St. Louis',          'MN', 47.58, -92.47),
('27139', 'Scott',              'MN', 44.65, -93.53),
('27141', 'Sherburne',          'MN', 45.45, -93.77),
('27143', 'Sibley',             'MN', 44.58, -94.23),
('27145', 'Stearns',            'MN', 45.55, -94.62),
('27147', 'Steele',             'MN', 44.02, -93.22),
('27149', 'Stevens',            'MN', 45.58, -96.00),
('27151', 'Swift',              'MN', 45.28, -95.68),
('27153', 'Todd',               'MN', 46.07, -94.90),
('27155', 'Traverse',           'MN', 45.77, -96.47),
('27157', 'Wabasha',            'MN', 44.28, -92.23),
('27159', 'Wadena',             'MN', 46.58, -95.08),
('27161', 'Waseca',             'MN', 44.00, -93.58),
('27163', 'Washington',         'MN', 45.03, -92.88),
('27165', 'Watonwan',           'MN', 43.97, -94.62),
('27167', 'Wilkin',             'MN', 46.35, -96.47),
('27169', 'Winona',             'MN', 44.00, -91.78),
('27171', 'Wright',             'MN', 45.17, -93.97),
('27173', 'Yellow Medicine',    'MN', 44.72, -95.87)
ON CONFLICT (fips_code) DO NOTHING;

-- =============================================================================
-- SEED DATA: Zip-to-County mapping (computed via nearest county centroid)
-- Uses the haversine_distance function from migration 000115.
-- Each MN zip code is assigned to the county whose centroid is nearest.
-- =============================================================================

INSERT INTO zip_counties (zip_code, county_id, is_primary)
SELECT DISTINCT ON (z.zip_code)
    z.zip_code,
    c.id,
    true
FROM zip_codes z
CROSS JOIN counties c
WHERE z.state = 'MN'
ORDER BY z.zip_code, haversine_distance(z.latitude, z.longitude, c.latitude, c.longitude)
ON CONFLICT DO NOTHING;

-- =============================================================================
-- SEED DATA: Row template configs (from CMS_SYSTEM_SPEC.md §7)
-- =============================================================================

INSERT INTO row_template_configs (slug, display_name, description, sort_order) VALUES
('hero-with-sidebar',       'Hero with sidebar',            'Full-width feature with stacked sidebar items',  1),
('hero-full',               'Hero full width',              'Single dominant feature story',                   2),
('three-column',            'Three column',                 'Mixed content row with three equal columns',      3),
('two-column-wide-narrow',  'Two column (wide + narrow)',   'Story with related sidebar',                      4),
('four-column',             'Four column grid',             'Dense card grid',                                 5),
('classifieds',             'Classifieds',                  'Compact listings (needs, offers)',                 6),
('ticker',                  'Ticker strip',                 'Horizontal strip of brief items',                 7),
('single-medium',           'Single card',                  'Standalone card (event, spotlight)',               8)
ON CONFLICT (slug) DO NOTHING;

-- =============================================================================
-- SEED DATA: Row template slot definitions
-- Each row template has slot groups: (slot_index, weight, count, accepts)
-- =============================================================================

-- hero-with-sidebar: 1 heavy + 3 light
INSERT INTO row_template_slots (row_template_config_id, slot_index, weight, count) VALUES
((SELECT id FROM row_template_configs WHERE slug = 'hero-with-sidebar'), 0, 'heavy', 1),
((SELECT id FROM row_template_configs WHERE slug = 'hero-with-sidebar'), 1, 'light', 3);

-- hero-full: 1 heavy
INSERT INTO row_template_slots (row_template_config_id, slot_index, weight, count) VALUES
((SELECT id FROM row_template_configs WHERE slug = 'hero-full'), 0, 'heavy', 1);

-- three-column: 3 medium
INSERT INTO row_template_slots (row_template_config_id, slot_index, weight, count) VALUES
((SELECT id FROM row_template_configs WHERE slug = 'three-column'), 0, 'medium', 3);

-- two-column-wide-narrow: 1 heavy + 1 medium
INSERT INTO row_template_slots (row_template_config_id, slot_index, weight, count) VALUES
((SELECT id FROM row_template_configs WHERE slug = 'two-column-wide-narrow'), 0, 'heavy', 1),
((SELECT id FROM row_template_configs WHERE slug = 'two-column-wide-narrow'), 1, 'medium', 1);

-- four-column: 4 medium
INSERT INTO row_template_slots (row_template_config_id, slot_index, weight, count) VALUES
((SELECT id FROM row_template_configs WHERE slug = 'four-column'), 0, 'medium', 4);

-- classifieds: 6 light
INSERT INTO row_template_slots (row_template_config_id, slot_index, weight, count) VALUES
((SELECT id FROM row_template_configs WHERE slug = 'classifieds'), 0, 'light', 6);

-- ticker: 8 light
INSERT INTO row_template_slots (row_template_config_id, slot_index, weight, count) VALUES
((SELECT id FROM row_template_configs WHERE slug = 'ticker'), 0, 'light', 8);

-- single-medium: 1 medium
INSERT INTO row_template_slots (row_template_config_id, slot_index, weight, count) VALUES
((SELECT id FROM row_template_configs WHERE slug = 'single-medium'), 0, 'medium', 1);

-- =============================================================================
-- SEED DATA: Post template configs (from CMS_SYSTEM_SPEC.md §8)
-- =============================================================================

INSERT INTO post_template_configs (slug, display_name, description, compatible_types, body_target, body_max, title_max, sort_order) VALUES
('feature',          'Feature',          'Premium editorial. Large typography, dramatic layout, image-heavy.',         '{story,event,spotlight}',                                400, 600, 80, 1),
('feature-reversed', 'Feature Reversed', 'Dark/high-contrast treatment. Used for urgent notices.',                     '{notice}',                                               200, 280, 60, 2),
('gazette',          'Gazette',          'Top-border tabbed frame, colored accent. Standard card.',                    '{story,notice,exchange,event,spotlight,reference}',       200, 280, 60, 3),
('ledger',           'Ledger',           'Left-border tabbed, classifieds feel, compact.',                             '{notice,exchange,event,reference}',                       120, 160, 50, 4),
('bulletin',         'Bulletin',         'Boxed card, community board feel.',                                          '{notice,exchange,event,reference,spotlight}',             180, 240, 60, 5),
('ticker',           'Ticker',           'Ultra-compact single-line. Title only, no body shown.',                      '{notice,exchange,event}',                                  0,   0, 50, 6),
('digest',           'Digest',           'Headline-only, no body text.',                                               '{story,notice,exchange}',                                  0,   0, 60, 7)
ON CONFLICT (slug) DO NOTHING;
