-- Add config_name column to agents for distinguishing public vs admin agents
ALTER TABLE agents ADD COLUMN config_name TEXT NOT NULL DEFAULT 'admin';

-- Only one active agent per config_name
CREATE UNIQUE INDEX idx_agents_config_name ON agents(config_name) WHERE is_active = true;
