-- Add support for user-submitted needs (not just scraped)

-- Add submission tracking to organization_needs
ALTER TABLE organization_needs
    ADD COLUMN submission_type TEXT DEFAULT 'scraped',
    ADD COLUMN submitted_by_volunteer_id UUID REFERENCES volunteers(id) ON DELETE SET NULL,
    ADD COLUMN location TEXT,
    ADD COLUMN submitted_from_ip INET;

-- Index for finding needs submitted by a volunteer
CREATE INDEX idx_organization_needs_submitted_by
    ON organization_needs(submitted_by_volunteer_id)
    WHERE submitted_by_volunteer_id IS NOT NULL;

COMMENT ON COLUMN organization_needs.submission_type IS 'How this need was created: scraped | user_submitted';
COMMENT ON COLUMN organization_needs.submitted_by_volunteer_id IS 'Volunteer who submitted this need (for user-submitted needs only)';
COMMENT ON COLUMN organization_needs.location IS 'Location/area for this need (e.g., "North Minneapolis", "Downtown St. Paul")';
COMMENT ON COLUMN organization_needs.submitted_from_ip IS 'IP address of submitter (for geolocation, spam prevention)';
