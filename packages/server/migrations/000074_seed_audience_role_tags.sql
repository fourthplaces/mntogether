-- Seed audience role tags
-- These tags describe WHO should engage with a listing and HOW

INSERT INTO tags (kind, value, display_name) VALUES
    ('audience_role', 'recipient', 'Recipient'),
    ('audience_role', 'donor', 'Donor'),
    ('audience_role', 'volunteer', 'Volunteer'),
    ('audience_role', 'participant', 'Participant')
ON CONFLICT (kind, value) DO NOTHING;

COMMENT ON TABLE tags IS 'audience_role tags: recipient (receiving services), donor (giving money/goods), volunteer (giving time), participant (attending events/groups)';
