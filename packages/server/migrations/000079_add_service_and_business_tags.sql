-- Add service_offered and business_model tag kinds
-- These enable richer categorization of listings

-- Seed common service_offered tags
INSERT INTO tags (kind, value, display_name) VALUES
    ('service_offered', 'meal-planning', 'Meal Planning'),
    ('service_offered', 'financial-skills', 'Financial Skills'),
    ('service_offered', 'self-care', 'Self-Care'),
    ('service_offered', 'transportation', 'Transportation'),
    ('service_offered', 'housing', 'Housing'),
    ('service_offered', 'legal-aid', 'Legal Aid'),
    ('service_offered', 'food-assistance', 'Food Assistance'),
    ('service_offered', 'job-training', 'Job Training'),
    ('service_offered', 'tutoring', 'Tutoring'),
    ('service_offered', 'mentoring', 'Mentoring'),
    ('service_offered', 'childcare', 'Childcare'),
    ('service_offered', 'healthcare', 'Healthcare'),
    ('service_offered', 'mental-health', 'Mental Health'),
    ('service_offered', 'immigration', 'Immigration'),
    ('service_offered', 'language-classes', 'Language Classes'),
    ('service_offered', 'citizenship', 'Citizenship'),
    ('service_offered', 'employment', 'Employment')
ON CONFLICT (kind, value) DO NOTHING;

-- Seed business_model tags
INSERT INTO tags (kind, value, display_name) VALUES
    ('business_model', 'nonprofit', 'Nonprofit'),
    ('business_model', 'social-enterprise', 'Social Enterprise'),
    ('business_model', 'donate-proceeds', 'Donates Proceeds'),
    ('business_model', 'community-owned', 'Community Owned')
ON CONFLICT (kind, value) DO NOTHING;

-- Seed org_leadership tags for ownership types
INSERT INTO tags (kind, value, display_name) VALUES
    ('org_leadership', 'immigrant-owned', 'Immigrant-Owned'),
    ('org_leadership', 'refugee-owned', 'Refugee-Owned'),
    ('org_leadership', 'woman-owned', 'Woman-Owned'),
    ('org_leadership', 'veteran-owned', 'Veteran-Owned'),
    ('org_leadership', 'bipoc-owned', 'BIPOC-Owned')
ON CONFLICT (kind, value) DO NOTHING;

-- Seed additional audience_role tags
INSERT INTO tags (kind, value, display_name) VALUES
    ('audience_role', 'customer', 'Customer'),
    ('audience_role', 'job-seeker', 'Job Seeker')
ON CONFLICT (kind, value) DO NOTHING;
