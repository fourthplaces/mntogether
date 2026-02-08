-- Create tag_kinds configuration table
-- Tracks which tag kinds exist and which resource types they apply to

CREATE TABLE tag_kinds (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    slug TEXT NOT NULL UNIQUE,
    display_name TEXT NOT NULL,
    description TEXT,
    allowed_resource_types TEXT[] NOT NULL DEFAULT '{}',
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- Seed existing tag kinds
INSERT INTO tag_kinds (slug, display_name, description, allowed_resource_types) VALUES
    ('audience_role', 'Audience Role', 'Who is this resource for (e.g., recipient, donor, volunteer)', ARRAY['post', 'website']),
    ('population', 'Population', 'Target populations served (e.g., seniors, refugees, youth)', ARRAY['post', 'website', 'provider']),
    ('community_served', 'Community Served', 'Cultural communities served (e.g., Somali, Hmong, Latino)', ARRAY['post', 'website', 'provider']),
    ('service_offered', 'Service Offered', 'Types of services offered (e.g., legal aid, food assistance)', ARRAY['post', 'website']),
    ('post_type', 'Post Type', 'Classification of post (e.g., service, business, event)', ARRAY['post']),
    ('org_leadership', 'Organization Leadership', 'Leadership identity (e.g., immigrant-owned, woman-owned)', ARRAY['post', 'website']),
    ('business_model', 'Business Model', 'Business structure (e.g., nonprofit, social enterprise)', ARRAY['post', 'website']),
    ('service_area', 'Service Area', 'Geographic areas served (e.g., Twin Cities, statewide)', ARRAY['post', 'website', 'provider']),
    ('provider_category', 'Provider Category', 'Provider role type (e.g., therapist, wellness coach)', ARRAY['provider']),
    ('provider_specialty', 'Provider Specialty', 'Provider specialization areas (e.g., grief, anxiety)', ARRAY['provider']),
    ('provider_language', 'Provider Language', 'Languages spoken by provider', ARRAY['provider']),
    ('service_language', 'Service Language', 'Languages offered by a service', ARRAY['post', 'website']),
    ('verification_source', 'Verification Source', 'Source of verification for organizations', ARRAY['website']),
    ('with_agent', 'With Agent', 'AI agent configuration for containers', ARRAY['container']);
