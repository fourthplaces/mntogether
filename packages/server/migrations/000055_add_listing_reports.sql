-- Add listing reporting system for moderation
-- Allows users to flag problematic listings for admin review

CREATE TABLE listing_reports (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  listing_id UUID NOT NULL REFERENCES listings(id) ON DELETE CASCADE,

  -- Reporter info (NULL for anonymous reports)
  reported_by UUID REFERENCES members(id) ON DELETE SET NULL,
  reporter_email TEXT,  -- For anonymous reports

  -- Report details
  reason TEXT NOT NULL,  -- Free-form explanation
  category TEXT NOT NULL CHECK (category IN (
    'inappropriate_content',
    'spam',
    'misleading_information',
    'duplicate',
    'outdated',
    'offensive',
    'other'
  )),

  -- Resolution tracking
  status TEXT NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'resolved', 'dismissed')),
  resolved_by UUID REFERENCES members(id) ON DELETE SET NULL,
  resolved_at TIMESTAMPTZ,
  resolution_notes TEXT,

  -- Action taken on the listing
  action_taken TEXT CHECK (action_taken IN ('listing_deleted', 'listing_rejected', 'listing_updated', 'no_action')),

  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Indexes for common queries
CREATE INDEX idx_listing_reports_listing ON listing_reports(listing_id);
CREATE INDEX idx_listing_reports_status ON listing_reports(status) WHERE status = 'pending';
CREATE INDEX idx_listing_reports_created ON listing_reports(created_at DESC);

-- View for admin dashboard: reports with listing details
CREATE OR REPLACE VIEW listing_reports_with_details AS
SELECT
  r.id,
  r.listing_id,
  r.reason,
  r.category,
  r.status,
  r.created_at,
  r.resolved_at,
  r.resolution_notes,
  r.action_taken,
  l.title as listing_title,
  l.organization_name,
  l.listing_type,
  l.status as listing_status,
  COUNT(*) OVER (PARTITION BY r.listing_id) as report_count_for_listing
FROM listing_reports r
INNER JOIN listings l ON l.id = r.listing_id
ORDER BY r.created_at DESC;
