-- Unify post ownership: replace submitted_by_admin_id + agent_id with single submitted_by_id

-- Add unified submitted_by_id column
ALTER TABLE posts ADD COLUMN submitted_by_id UUID REFERENCES members(id);

-- Backfill from agent_id (agent posts → agent.member_id)
UPDATE posts SET submitted_by_id = agents.member_id
FROM agents WHERE posts.agent_id = agents.id;

-- Backfill from submitted_by_admin_id (admin posts)
UPDATE posts SET submitted_by_id = submitted_by_admin_id
WHERE submitted_by_admin_id IS NOT NULL AND submitted_by_id IS NULL;

-- Drop old columns
ALTER TABLE posts DROP COLUMN submitted_by_admin_id;
ALTER TABLE posts DROP COLUMN agent_id;

-- Index for agent-scoped queries
CREATE INDEX idx_posts_submitted_by_id ON posts(submitted_by_id) WHERE deleted_at IS NULL;
