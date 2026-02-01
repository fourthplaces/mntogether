-- Create post_contacts table for storing contact information for posts
-- This is separate from the polymorphic contacts table used for organizations

CREATE TABLE post_contacts (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    post_id UUID NOT NULL REFERENCES posts(id) ON DELETE CASCADE,
    contact_type TEXT NOT NULL CHECK (contact_type IN ('phone', 'email', 'website', 'address')),
    contact_value TEXT NOT NULL,
    contact_label TEXT,
    display_order INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_post_contacts_post_id ON post_contacts(post_id);
CREATE INDEX idx_post_contacts_type ON post_contacts(contact_type);
