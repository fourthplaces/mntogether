-- Fix: Rename domain_research_homepage to website_research_homepage
-- This was missed in the 000057 migration

ALTER TABLE domain_research_homepage RENAME TO website_research_homepage;
ALTER TABLE website_research_homepage RENAME COLUMN domain_research_id TO website_research_id;

-- Update foreign key constraint
ALTER TABLE website_research_homepage DROP CONSTRAINT IF EXISTS domain_research_homepage_domain_research_id_fkey;
ALTER TABLE website_research_homepage
    ADD CONSTRAINT website_research_homepage_website_research_id_fkey
    FOREIGN KEY (website_research_id) REFERENCES website_research(id) ON DELETE CASCADE;

-- Update index
DROP INDEX IF EXISTS idx_domain_research_homepage_research_id;
CREATE INDEX idx_website_research_homepage_research_id ON website_research_homepage(website_research_id);
