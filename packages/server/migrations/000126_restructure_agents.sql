-- Restructure agents into base table + role-specific config tables.
-- Existing chatbot agents become role='assistant' with config in agent_assistant_configs.
-- New curator agents get config in agent_curator_configs.

-- Step 1: Add role column to existing agents table
ALTER TABLE agents ADD COLUMN role TEXT NOT NULL DEFAULT 'assistant';
ALTER TABLE agents ADD CONSTRAINT agents_role_check CHECK (role IN ('assistant', 'curator'));

-- Step 2: Create assistant config table
CREATE TABLE agent_assistant_configs (
    agent_id UUID PRIMARY KEY REFERENCES agents(id) ON DELETE CASCADE,
    preamble TEXT NOT NULL DEFAULT '',
    config_name TEXT NOT NULL DEFAULT 'admin'
);

-- Step 3: Migrate existing assistant data into config table
INSERT INTO agent_assistant_configs (agent_id, preamble, config_name)
SELECT id, preamble, config_name FROM agents;

-- Step 4: Drop assistant-specific columns from base table
ALTER TABLE agents DROP COLUMN preamble;
ALTER TABLE agents DROP COLUMN config_name;

-- Step 5: Add status column (replaces is_active for all roles)
ALTER TABLE agents ADD COLUMN status TEXT NOT NULL DEFAULT 'active';
ALTER TABLE agents ADD CONSTRAINT agents_status_check CHECK (status IN ('draft', 'active', 'paused'));
UPDATE agents SET status = CASE WHEN is_active THEN 'active' ELSE 'paused' END;
ALTER TABLE agents DROP COLUMN is_active;

-- Step 6: Move unique index to assistant config table
-- Original: unique config_name among active agents (partial index on is_active)
-- Now: global uniqueness on config_name (only 2 values exist: 'admin', 'public')
DROP INDEX IF EXISTS idx_agents_config_name;
CREATE UNIQUE INDEX idx_agent_assistant_configs_config_name
    ON agent_assistant_configs(config_name);

-- Step 7: Create curator config table
CREATE TABLE agent_curator_configs (
    agent_id UUID PRIMARY KEY REFERENCES agents(id) ON DELETE CASCADE,
    purpose TEXT NOT NULL DEFAULT '',
    audience_roles TEXT[] NOT NULL DEFAULT '{}',
    schedule_discover TEXT,
    schedule_monitor TEXT
);
