-- Migration 000053: Seed Initial Agents
--
-- Seeds the agents table with initial autonomous agents focused on
-- Twin Cities, Minnesota community resources.
--
-- These agents automatically:
-- 1. Search for domains via Tavily (using query_template)
-- 2. Auto-scrape discovered domains via Firecrawl
-- 3. Extract listings via AI (using extraction_instructions)
-- 4. Auto-approve domains when listings are found

INSERT INTO agents (
    name,
    query_template,
    description,
    location_context,
    service_area_tags,
    search_frequency_hours,
    max_results,
    extraction_instructions,
    system_prompt,
    enabled,
    auto_approve_domains,
    auto_scrape,
    auto_create_listings
)
VALUES
    (
        'Legal Aid for Immigrants',
        'legal aid immigrants refugees "immigration lawyer" pro bono {location}',
        'Legal services for immigrants and refugees',
        'Minnesota',
        ARRAY['legal_aid', 'immigration', 'refugee_services'],
        24,
        5,
        'Extract legal services for immigrants including: eligibility requirements (income limits, visa status), languages offered, types of cases handled (asylum, citizenship, deportation defense), contact information (phone, email, website), office hours, and fee structure (free, sliding scale, pro bono). Note if interpretation services are available.',
        'You are an expert at identifying legal aid services for immigrant communities. Focus on extracting comprehensive eligibility criteria, language accessibility, and specific immigration law services offered. Prioritize information about free or low-cost services.',
        true,
        true,
        true,
        true
    ),
    (
        'Volunteer Opportunities',
        'volunteer opportunities community service {location}',
        'Places to volunteer in the local community',
        'Minnesota',
        ARRAY['volunteering', 'community_engagement'],
        24,
        5,
        'Extract volunteer opportunities including: time commitment required, skills needed, age restrictions, background check requirements, training provided, types of activities, scheduling flexibility, and contact information. Note if remote volunteering is available.',
        'You are an expert at identifying volunteer opportunities. Extract detailed requirements for volunteers, types of support provided, and logistics like time commitment and scheduling. Highlight opportunities suitable for volunteers of all skill levels.',
        true,
        true,
        true,
        true
    ),
    (
        'Food Banks and Pantries',
        'food banks pantries free meals emergency food {location}',
        'Emergency food assistance programs',
        'Minnesota',
        ARRAY['food_assistance', 'emergency_services'],
        24,
        5,
        'Extract food assistance programs including: eligibility requirements, documentation needed, hours of operation, types of food provided (groceries, hot meals, dietary accommodations), frequency limits, languages spoken, delivery/pickup options, and contact information.',
        'You are an expert at identifying food assistance resources. Extract comprehensive details about eligibility, access methods, and available food types. Prioritize information about requirements, hours, and how people can get help immediately.',
        true,
        true,
        true,
        true
    ),
    (
        'Small Business Support',
        'small business support grants loans {location}',
        'Resources for supporting local small businesses',
        'Minnesota',
        ARRAY['economic_development', 'business_support'],
        48,
        5,
        'Extract small business support programs including: eligibility requirements, application process, funding amounts, business stage requirements, industry focus, deadlines, technical assistance offered, and contact information.',
        'You are an expert at identifying small business support programs. Extract details about funding amounts, eligibility criteria, application requirements, and support services. Focus on actionable information for business owners.',
        true,
        true,
        true,
        true
    ),
    (
        'Housing Assistance',
        'housing assistance affordable rent emergency shelter {location}',
        'Housing and rental assistance programs',
        'Minnesota',
        ARRAY['housing', 'emergency_services'],
        24,
        5,
        'Extract housing assistance programs including: eligibility requirements (income limits, family size), types of assistance (rent, deposits, utilities, emergency shelter), application process, waiting list status, documentation needed, contact information, and crisis hotlines.',
        'You are an expert at identifying housing assistance resources. Extract comprehensive eligibility criteria, types of assistance available, and application procedures. Prioritize emergency resources and immediate help options.',
        true,
        true,
        true,
        true
    ),
    (
        'Mental Health Services',
        'mental health counseling therapy free low-cost crisis {location}',
        'Accessible mental health and counseling services',
        'Minnesota',
        ARRAY['mental_health', 'healthcare'],
        48,
        5,
        'Extract mental health services including: eligibility requirements, types of therapy offered, languages available, sliding scale fees or free services, insurance accepted, crisis services, age groups served, telehealth availability, and contact information.',
        'You are an expert at identifying mental health resources. Extract details about accessibility, cost, languages, and crisis services. Prioritize free or low-cost options and immediate crisis support.',
        true,
        true,
        true,
        true
    ),
    (
        'Job Training Programs',
        'job training workforce development skills {location}',
        'Employment training and workforce development',
        'Minnesota',
        ARRAY['employment', 'education', 'job_training'],
        48,
        5,
        'Extract job training programs including: eligibility requirements, skills taught, program duration, cost or stipends, certifications earned, job placement assistance, languages available, schedule flexibility, and contact information.',
        'You are an expert at identifying workforce development programs. Extract comprehensive details about training offerings, outcomes, costs, and support services. Focus on accessible programs with strong job placement records.',
        true,
        true,
        true,
        true
    ),
    (
        'Senior Support Services',
        'senior services elderly support companionship meals transportation {location}',
        'Support services for senior citizens',
        'Minnesota',
        ARRAY['senior_services', 'elderly_care'],
        24,
        5,
        'Extract senior support services including: eligibility requirements (age, income), types of services (meals, transportation, companionship, home care), cost structure, service area, languages available, and contact information. Note if caregiver support is available.',
        'You are an expert at identifying senior support services. Extract comprehensive details about service types, eligibility, costs, and geographic coverage. Prioritize accessible services that support aging in place.',
        true,
        true,
        true,
        true
    );

-- Add comment
COMMENT ON TABLE agents IS 'Seeded with 8 autonomous agents focused on Twin Cities community resources. Each agent searches, scrapes, and extracts automatically.';
