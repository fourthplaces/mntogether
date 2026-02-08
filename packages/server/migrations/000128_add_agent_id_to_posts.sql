-- Add agent_id to posts so posts can be owned by an agent.
-- Nullable because existing posts predate agents.

ALTER TABLE posts ADD COLUMN agent_id UUID REFERENCES agents(id);
CREATE INDEX idx_posts_agent_id ON posts(agent_id);
