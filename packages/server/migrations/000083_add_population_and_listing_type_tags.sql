-- Add population and listing_type tag kinds for richer categorization

-- Population tags - who the listing serves
INSERT INTO tags (kind, value, display_name) VALUES
    ('population', 'disabilities', 'People with Disabilities'),
    ('population', 'brain-injury', 'Brain Injury'),
    ('population', 'seniors', 'Seniors'),
    ('population', 'refugees', 'Refugees'),
    ('population', 'immigrants', 'Immigrants'),
    ('population', 'youth', 'Youth'),
    ('population', 'families', 'Families'),
    ('population', 'veterans', 'Veterans'),
    ('population', 'homeless', 'People Experiencing Homelessness'),
    ('population', 'low-income', 'Low Income')
ON CONFLICT (kind, value) DO NOTHING;

-- Listing type tags - category of listing
INSERT INTO tags (kind, value, display_name) VALUES
    ('listing_type', 'service', 'Service'),
    ('listing_type', 'business', 'Business'),
    ('listing_type', 'event', 'Event'),
    ('listing_type', 'opportunity', 'Opportunity')
ON CONFLICT (kind, value) DO NOTHING;

-- Service area tags - geographic coverage
INSERT INTO tags (kind, value, display_name) VALUES
    ('service_area', 'twin-cities', 'Twin Cities Metro'),
    ('service_area', 'minneapolis', 'Minneapolis'),
    ('service_area', 'st-paul', 'St. Paul'),
    ('service_area', 'st-cloud', 'St. Cloud Area'),
    ('service_area', 'rochester', 'Rochester Area'),
    ('service_area', 'duluth', 'Duluth Area'),
    ('service_area', 'statewide', 'Statewide Minnesota'),
    ('service_area', 'central-mn', 'Central Minnesota')
ON CONFLICT (kind, value) DO NOTHING;

-- Community served tags (ethnic/cultural communities)
INSERT INTO tags (kind, value, display_name) VALUES
    ('community_served', 'somali', 'Somali'),
    ('community_served', 'hmong', 'Hmong'),
    ('community_served', 'karen', 'Karen'),
    ('community_served', 'latino', 'Latino/Hispanic'),
    ('community_served', 'east-african', 'East African'),
    ('community_served', 'southeast-asian', 'Southeast Asian'),
    ('community_served', 'arabic-speaking', 'Arabic Speaking')
ON CONFLICT (kind, value) DO NOTHING;
