-- Add tags to containers for agent configuration
-- Tags like { "with-agent": "default" } control agent behavior

ALTER TABLE containers ADD COLUMN IF NOT EXISTS tags JSONB DEFAULT '{}';

-- Index for querying containers by tag
CREATE INDEX IF NOT EXISTS idx_containers_tags ON containers USING GIN (tags);
