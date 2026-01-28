-- Create identifiers table for phone-based authentication
-- Links members to their hashed phone numbers with admin status

CREATE TABLE identifiers (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    member_id UUID NOT NULL,
    phone_hash VARCHAR(64) NOT NULL UNIQUE,
    is_admin BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT fk_member
        FOREIGN KEY (member_id)
        REFERENCES members(id)
        ON DELETE CASCADE
);

-- Index for fast lookups by phone hash
CREATE INDEX idx_identifiers_phone_hash ON identifiers(phone_hash);

-- Index for member lookups
CREATE INDEX idx_identifiers_member_id ON identifiers(member_id);

-- Trigger to update updated_at timestamp
CREATE OR REPLACE FUNCTION update_identifiers_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trigger_update_identifiers_updated_at
    BEFORE UPDATE ON identifiers
    FOR EACH ROW
    EXECUTE FUNCTION update_identifiers_updated_at();
