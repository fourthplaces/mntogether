-- Drop dead agents domain: tables, tag kind, and submission_type constraint cleanup.

-- Drop agent tables (order matters: config table references agents)
DROP TABLE IF EXISTS agent_assistant_configs;
DROP TABLE IF EXISTS agents;

-- Remove 'with_agent' tag kind (and any tags under it)
DELETE FROM tags WHERE kind = 'with_agent';
DELETE FROM tag_kinds WHERE slug = 'with_agent';

-- Tighten submission_type check constraint to remove 'agent'
ALTER TABLE posts DROP CONSTRAINT IF EXISTS listings_submission_type_check;
ALTER TABLE posts ADD CONSTRAINT listings_submission_type_check
  CHECK (submission_type IN ('scraped', 'admin', 'org_submitted', 'revision'));

-- Clear any posts that had submission_type = 'agent' (set to 'admin')
UPDATE posts SET submission_type = 'admin' WHERE submission_type = 'agent';
