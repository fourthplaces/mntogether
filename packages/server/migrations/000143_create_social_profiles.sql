CREATE TABLE social_profiles (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID NOT NULL REFERENCES organizations(id),
    platform TEXT NOT NULL,
    handle TEXT NOT NULL,
    url TEXT,
    scrape_frequency_hours INT NOT NULL DEFAULT 24,
    last_scraped_at TIMESTAMPTZ,
    active BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (platform, handle)
);

CREATE INDEX idx_social_profiles_organization_id ON social_profiles(organization_id);
CREATE INDEX idx_social_profiles_platform ON social_profiles(platform);
