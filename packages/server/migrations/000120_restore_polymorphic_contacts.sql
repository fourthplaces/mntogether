-- Restore polymorphic contacts table (originally 000071, dropped in 000118)
CREATE TABLE contacts (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    contactable_type TEXT NOT NULL,
    contactable_id UUID NOT NULL,
    contact_type TEXT NOT NULL CHECK (contact_type IN (
        'phone', 'email', 'website', 'address', 'booking_url', 'social'
    )),
    contact_value TEXT NOT NULL,
    contact_label TEXT,
    is_public BOOLEAN DEFAULT true,
    display_order INTEGER DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(contactable_type, contactable_id, contact_type, contact_value)
);

CREATE INDEX idx_contacts_entity ON contacts(contactable_type, contactable_id);
CREATE INDEX idx_contacts_type ON contacts(contact_type);

-- Migrate existing post_contacts data
INSERT INTO contacts (contactable_type, contactable_id, contact_type, contact_value, contact_label, display_order, created_at)
SELECT 'post', post_id, contact_type, contact_value, contact_label, display_order, created_at
FROM post_contacts
ON CONFLICT DO NOTHING;

-- Drop old table
DROP TABLE post_contacts;
