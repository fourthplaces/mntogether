-- Add 'agent' to the submission_type check constraint for AI consultant-created posts.
ALTER TABLE posts DROP CONSTRAINT IF EXISTS listings_submission_type_check;
ALTER TABLE posts ADD CONSTRAINT listings_submission_type_check
  CHECK (submission_type IN ('scraped', 'admin', 'org_submitted', 'revision', 'agent'));
