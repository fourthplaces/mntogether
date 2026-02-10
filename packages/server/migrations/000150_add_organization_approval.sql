ALTER TABLE organizations
  ADD COLUMN status TEXT NOT NULL DEFAULT 'pending_review',
  ADD COLUMN submitted_by UUID,
  ADD COLUMN submitter_type TEXT,
  ADD COLUMN submission_context TEXT,
  ADD COLUMN reviewed_by UUID,
  ADD COLUMN reviewed_at TIMESTAMPTZ,
  ADD COLUMN rejection_reason TEXT;

-- Existing orgs are already vetted — mark as approved
UPDATE organizations SET status = 'approved', submitter_type = 'admin';

-- Indexes
CREATE INDEX idx_organizations_status ON organizations(status);
CREATE INDEX idx_organizations_pending ON organizations(status) WHERE status = 'pending_review';
