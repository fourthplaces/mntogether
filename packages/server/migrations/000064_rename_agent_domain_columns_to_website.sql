-- Migration: Rename domain-related columns in agents table to website
-- Continuing the domains → websites naming refactor

-- Rename columns in agents table
ALTER TABLE agents RENAME COLUMN auto_approve_domains TO auto_approve_websites;
ALTER TABLE agents RENAME COLUMN total_domains_discovered TO total_websites_discovered;
ALTER TABLE agents RENAME COLUMN total_domains_approved TO total_websites_approved;

-- Add comment documenting the rename
COMMENT ON COLUMN agents.auto_approve_websites IS 'Whether to auto-approve websites when listings are extracted';
COMMENT ON COLUMN agents.total_websites_discovered IS 'Count of websites discovered by this agent';
COMMENT ON COLUMN agents.total_websites_approved IS 'Count of websites approved from discoveries';
