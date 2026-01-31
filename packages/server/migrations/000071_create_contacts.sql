-- Polymorphic contacts table (like taggables pattern)
-- Stores contact information for any entity type: organization, listing, provider
CREATE TABLE contacts (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    contactable_type TEXT NOT NULL,  -- 'organization', 'listing', 'provider'
    contactable_id UUID NOT NULL,
    contact_type TEXT NOT NULL CHECK (contact_type IN (
        'phone', 'email', 'website', 'address', 'booking_url', 'social'
    )),
    contact_value TEXT NOT NULL,
    contact_label TEXT,              -- 'Office', 'Mobile', 'LinkedIn'
    is_public BOOLEAN DEFAULT true,
    display_order INTEGER DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    UNIQUE(contactable_type, contactable_id, contact_type, contact_value)
);

CREATE INDEX idx_contacts_entity ON contacts(contactable_type, contactable_id);
CREATE INDEX idx_contacts_type ON contacts(contact_type);
