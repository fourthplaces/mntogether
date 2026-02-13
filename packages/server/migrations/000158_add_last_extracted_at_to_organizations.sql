-- Add last_extracted_at to organizations for tracking when org-level extraction last ran.
-- Used by OrgExtractionScheduler to find orgs needing re-extraction.
ALTER TABLE organizations ADD COLUMN last_extracted_at TIMESTAMP WITH TIME ZONE;
