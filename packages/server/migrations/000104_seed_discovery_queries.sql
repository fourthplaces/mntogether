-- Seed existing hardcoded discovery queries into the database
-- These were previously in posts/effects/discovery.rs DISCOVERY_QUERIES constant

-- Services - help for people in need
INSERT INTO discovery_queries (query_text, category) VALUES
    ('community resources social services {location}', 'services'),
    ('food assistance food shelf food bank {location}', 'services'),
    ('housing assistance rental help {location}', 'services'),
    ('emergency shelter homeless services {location}', 'services'),
    ('utility assistance bill help {location}', 'services'),
    ('healthcare free clinic sliding scale {location}', 'services'),
    ('mental health services counseling {location}', 'services'),
    ('childcare assistance programs {location}', 'services'),
    ('senior services elderly assistance {location}', 'services'),
    ('disability services support {location}', 'services');

-- Professionals - people who help
INSERT INTO discovery_queries (query_text, category) VALUES
    ('immigration lawyer attorney {location}', 'professionals'),
    ('pro bono legal services {location}', 'professionals'),
    ('nonprofit legal aid {location}', 'professionals'),
    ('immigration help DACA {location}', 'professionals');

-- Businesses - places to support
INSERT INTO discovery_queries (query_text, category) VALUES
    ('immigrant owned business {location}', 'businesses'),
    ('refugee owned restaurant {location}', 'businesses'),
    ('minority owned business {location}', 'businesses'),
    ('social enterprise {location}', 'businesses');

-- Opportunities - things to do
INSERT INTO discovery_queries (query_text, category) VALUES
    ('volunteer opportunities nonprofit {location}', 'opportunities'),
    ('community service opportunities {location}', 'opportunities'),
    ('tutoring mentoring volunteer {location}', 'opportunities'),
    ('refugee resettlement volunteer {location}', 'opportunities');

-- Events & Fundraising
INSERT INTO discovery_queries (query_text, category) VALUES
    ('community fundraising event {location}', 'events'),
    ('nonprofit fundraiser gala {location}', 'events'),
    ('charity event benefit {location}', 'events'),
    ('community benefit dinner {location}', 'events'),
    ('immigrant community event {location}', 'events'),
    ('cultural celebration festival {location}', 'events');

-- Global filter rules (apply to all queries)
INSERT INTO discovery_filter_rules (query_id, rule_text, sort_order) VALUES
    (NULL, 'Omit government websites (.gov domains, city/county/state agencies)', 1),
    (NULL, 'Skip generic directory or aggregator websites that list other organizations but do not provide services themselves', 2),
    (NULL, 'Skip social media profiles (facebook.com, twitter.com, instagram.com, linkedin.com)', 3);
