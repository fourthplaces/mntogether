-- Create agents table: AI agents with proper member identity.
-- Each agent gets a real member row so messages.author_id FK works.

CREATE TABLE agents (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    member_id UUID NOT NULL UNIQUE REFERENCES members(id) ON DELETE CASCADE,
    display_name TEXT NOT NULL,
    preamble TEXT NOT NULL DEFAULT '',
    is_active BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Seed the default admin agent
DO $$
DECLARE
    agent_member_id UUID;
BEGIN
    INSERT INTO members (expo_push_token, searchable_text, active, notification_count_this_week)
    VALUES ('agent:default', 'AI Admin Assistant', true, 0)
    ON CONFLICT (expo_push_token) DO UPDATE SET searchable_text = EXCLUDED.searchable_text
    RETURNING id INTO agent_member_id;

    INSERT INTO agents (member_id, display_name, preamble)
    VALUES (
        agent_member_id,
        'MN Together Assistant',
        'You are an admin assistant for MN Together, a resource-sharing platform.
You can help administrators:
- Approve or reject listings
- Scrape websites for new resources
- Generate website assessments
- Search and filter listings
- Manage organizations

Be helpful and proactive. If an admin asks to do something, use the appropriate tool.'
    )
    ON CONFLICT (member_id) DO NOTHING;
END $$;
