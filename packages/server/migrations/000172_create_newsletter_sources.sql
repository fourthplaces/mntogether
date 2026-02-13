-- Newsletter source support: detected forms + newsletter subscriptions
-- Follows class table inheritance pattern from 000149

CREATE TABLE detected_newsletter_forms (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    website_source_id UUID NOT NULL REFERENCES website_sources(id) ON DELETE CASCADE,
    form_url TEXT NOT NULL,
    form_type TEXT NOT NULL DEFAULT 'unknown',
    requires_extra_fields BOOLEAN NOT NULL DEFAULT false,
    extra_fields_detected JSONB NOT NULL DEFAULT '[]',
    status TEXT NOT NULL DEFAULT 'detected',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(website_source_id, form_url)
);

CREATE TABLE newsletter_sources (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    source_id UUID NOT NULL UNIQUE REFERENCES sources(id) ON DELETE CASCADE,
    ingest_email TEXT NOT NULL UNIQUE,
    signup_form_url TEXT NOT NULL,
    subscription_status TEXT NOT NULL DEFAULT 'detected',
    confirmation_link TEXT,
    confirmation_email_received_at TIMESTAMPTZ,
    expected_sender_domain TEXT,
    last_newsletter_received_at TIMESTAMPTZ,
    newsletters_received_count INT NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_newsletter_sources_status ON newsletter_sources(subscription_status);
CREATE INDEX idx_newsletter_sources_ingest_email ON newsletter_sources(ingest_email);
CREATE INDEX idx_detected_newsletter_forms_website ON detected_newsletter_forms(website_source_id);
