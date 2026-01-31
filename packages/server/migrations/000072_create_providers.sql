-- Providers table for professional directory
-- Stores wellness coaches, therapists, counselors, and other service providers
CREATE TABLE providers (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    -- Profile
    name TEXT NOT NULL,
    bio TEXT,
    why_statement TEXT,
    headline TEXT,
    profile_image_url TEXT,

    -- Links
    member_id UUID REFERENCES members(id) ON DELETE SET NULL,
    website_id UUID REFERENCES websites(id) ON DELETE SET NULL,

    -- Location
    location TEXT,
    latitude DOUBLE PRECISION,
    longitude DOUBLE PRECISION,
    service_radius_km INTEGER,

    -- Service modes
    offers_in_person BOOLEAN DEFAULT false,
    offers_remote BOOLEAN DEFAULT false,

    -- Availability
    accepting_clients BOOLEAN DEFAULT true,

    -- Approval workflow
    status TEXT NOT NULL DEFAULT 'pending_review'
        CHECK (status IN ('pending_review', 'approved', 'rejected', 'suspended')),
    submitted_by UUID REFERENCES members(id),
    reviewed_by UUID REFERENCES members(id),
    reviewed_at TIMESTAMPTZ,
    rejection_reason TEXT,

    -- Matching
    embedding vector(1536),

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_providers_status ON providers(status);
CREATE INDEX idx_providers_member ON providers(member_id) WHERE member_id IS NOT NULL;
CREATE INDEX idx_providers_accepting ON providers(accepting_clients)
    WHERE status = 'approved' AND accepting_clients = true;
CREATE INDEX idx_providers_embedding ON providers USING hnsw (embedding vector_cosine_ops)
    WHERE embedding IS NOT NULL AND status = 'approved';
