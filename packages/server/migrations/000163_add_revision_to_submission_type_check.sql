-- Add 'revision' to the submission_type check constraint.
-- The constraint was created in migration 000032 with only ('scraped', 'admin', 'org_submitted'),
-- but revision posts need submission_type = 'revision'.
ALTER TABLE posts DROP CONSTRAINT IF EXISTS listings_submission_type_check;
ALTER TABLE posts ADD CONSTRAINT listings_submission_type_check
  CHECK (submission_type IN ('scraped', 'admin', 'org_submitted', 'revision'));
