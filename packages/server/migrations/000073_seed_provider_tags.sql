-- Seed provider categories
INSERT INTO tags (kind, value, display_name) VALUES
    ('provider_category', 'wellness_coach', 'Wellness Coach'),
    ('provider_category', 'therapist', 'Therapist'),
    ('provider_category', 'counselor', 'Counselor'),
    ('provider_category', 'career_coach', 'Career Coach'),
    ('provider_category', 'peer_support', 'Peer Support Specialist'),
    ('provider_category', 'life_coach', 'Life Coach'),
    ('provider_category', 'financial_coach', 'Financial Coach'),
    ('provider_category', 'doula', 'Doula'),
    ('provider_category', 'social_worker', 'Social Worker'),
    ('provider_category', 'navigator', 'Navigator')
ON CONFLICT (kind, value) DO NOTHING;

-- Seed provider specialties
INSERT INTO tags (kind, value, display_name) VALUES
    ('provider_specialty', 'anxiety', 'Anxiety'),
    ('provider_specialty', 'depression', 'Depression'),
    ('provider_specialty', 'grief', 'Grief & Loss'),
    ('provider_specialty', 'trauma', 'Trauma & PTSD'),
    ('provider_specialty', 'addiction', 'Addiction & Recovery'),
    ('provider_specialty', 'relationships', 'Relationships'),
    ('provider_specialty', 'career_transition', 'Career Transitions'),
    ('provider_specialty', 'stress_management', 'Stress Management'),
    ('provider_specialty', 'self_esteem', 'Self-Esteem'),
    ('provider_specialty', 'parenting', 'Parenting'),
    ('provider_specialty', 'life_transition', 'Life Transitions'),
    ('provider_specialty', 'burnout', 'Burnout'),
    ('provider_specialty', 'immigration', 'Immigration Support'),
    ('provider_specialty', 'youth', 'Youth & Adolescents'),
    ('provider_specialty', 'seniors', 'Seniors & Aging'),
    ('provider_specialty', 'lgbtq', 'LGBTQ+'),
    ('provider_specialty', 'veterans', 'Veterans'),
    ('provider_specialty', 'cultural_identity', 'Cultural Identity')
ON CONFLICT (kind, value) DO NOTHING;

-- Seed provider languages
INSERT INTO tags (kind, value, display_name) VALUES
    ('provider_language', 'en', 'English'),
    ('provider_language', 'es', 'Spanish'),
    ('provider_language', 'hmn', 'Hmong'),
    ('provider_language', 'so', 'Somali'),
    ('provider_language', 'vi', 'Vietnamese'),
    ('provider_language', 'am', 'Amharic'),
    ('provider_language', 'or', 'Oromo'),
    ('provider_language', 'kar', 'Karen'),
    ('provider_language', 'ar', 'Arabic'),
    ('provider_language', 'zh', 'Chinese (Mandarin)'),
    ('provider_language', 'ko', 'Korean'),
    ('provider_language', 'ru', 'Russian'),
    ('provider_language', 'fr', 'French'),
    ('provider_language', 'pt', 'Portuguese'),
    ('provider_language', 'sw', 'Swahili')
ON CONFLICT (kind, value) DO NOTHING;
