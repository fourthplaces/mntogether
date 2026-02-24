-- Fix: Rename domain_research_id to website_research_id in website_assessments
-- This was missed in the 000057 migration

ALTER TABLE website_assessments RENAME COLUMN domain_research_id TO website_research_id;

ALTER TABLE website_assessments DROP CONSTRAINT IF EXISTS domain_assessments_domain_research_id_fkey;
ALTER TABLE website_assessments
    ADD CONSTRAINT website_assessments_website_research_id_fkey
    FOREIGN KEY (website_research_id) REFERENCES website_research(id) ON DELETE SET NULL;

DROP INDEX IF EXISTS idx_domain_assessments_research_id;
CREATE INDEX idx_website_assessments_website_research_id ON website_assessments(website_research_id);
