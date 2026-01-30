-- Migration 000051: Seed Initial Search Topics
--
-- Seeds the search_topics table with initial community resource topics
-- focused on the Twin Cities, Minnesota area.

INSERT INTO search_topics (name, query_template, description, service_area_tags, search_frequency_hours, max_results)
VALUES
    (
        'Legal Aid - Immigrants',
        'legal aid immigrants refugees {location}',
        'Legal services for immigrants and refugees',
        ARRAY['legal_aid', 'immigration', 'refugee_services'],
        24,
        5
    ),
    (
        'Volunteer Opportunities',
        'volunteer opportunities community service {location}',
        'Places to volunteer in the local community',
        ARRAY['volunteering', 'community_engagement'],
        24,
        5
    ),
    (
        'Food Banks and Pantries',
        'food banks pantries free meals {location}',
        'Emergency food assistance programs',
        ARRAY['food_assistance', 'emergency_services'],
        24,
        5
    ),
    (
        'Small Business Support',
        'small business support grants loans {location}',
        'Resources for supporting local small businesses',
        ARRAY['economic_development', 'business_support'],
        48,
        5
    ),
    (
        'Donation Centers',
        'donation centers clothing furniture household {location}',
        'Places to donate goods and clothing',
        ARRAY['donations', 'community_resources'],
        48,
        5
    ),
    (
        'Housing Assistance',
        'housing assistance affordable rent emergency shelter {location}',
        'Housing and rental assistance programs',
        ARRAY['housing', 'emergency_services'],
        24,
        5
    ),
    (
        'Mental Health Services',
        'mental health counseling therapy free low-cost {location}',
        'Accessible mental health and counseling services',
        ARRAY['mental_health', 'healthcare'],
        48,
        5
    ),
    (
        'Job Training Programs',
        'job training workforce development skills {location}',
        'Employment training and workforce development',
        ARRAY['employment', 'education', 'job_training'],
        48,
        5
    );

-- Add comment
COMMENT ON TABLE search_topics IS 'Seeded with 8 initial search topics focused on Twin Cities community resources';
