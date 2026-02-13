-- Add 'agent' to the allowed submitter_type values for websites
ALTER TABLE websites DROP CONSTRAINT IF EXISTS domains_submitter_type_check;
ALTER TABLE websites ADD CONSTRAINT domains_submitter_type_check
  CHECK (submitter_type IN ('admin', 'public_user', 'system', 'agent'));
