-- Add domain approval workflow
-- Domains must be reviewed/approved before crawling

-- Add approval workflow columns to domains
ALTER TABLE domains ADD COLUMN IF NOT EXISTS (
  -- Approval workflow
  status TEXT NOT NULL DEFAULT 'pending_review' CHECK (status IN (
    'pending_review',   -- Submitted, waiting for admin review
    'approved',         -- Approved for crawling
    'rejected',         -- Admin rejected (spam, not relevant, etc.)
    'suspended'         -- Temporarily suspended (rate limiting, errors, etc.)
  )),

  -- Submission metadata
  submitted_by UUID REFERENCES members(id) ON DELETE SET NULL,
  submitter_type TEXT CHECK (submitter_type IN ('admin', 'public_user', 'system')),
  submission_context TEXT, -- Why they think this domain has resources

  -- Review metadata
  reviewed_by UUID REFERENCES members(id) ON DELETE SET NULL,
  reviewed_at TIMESTAMPTZ,
  rejection_reason TEXT,

  -- Crawling configuration
  max_crawl_depth INT DEFAULT 3, -- Prevent runaway crawling
  crawl_rate_limit_seconds INT DEFAULT 2, -- Politeness delay between requests

  -- Trust level (for auto-approval of URLs from this domain)
  is_trusted_domain BOOL DEFAULT false -- If true, URLs from this domain are auto-approved
);

-- Update existing domains to 'approved' (assume pre-existing are approved)
UPDATE domains SET status = 'approved' WHERE status IS NULL;

-- Set active=false as equivalent to suspended
UPDATE domains SET status = 'suspended' WHERE active = false;

-- Make status NOT NULL after backfill
ALTER TABLE domains ALTER COLUMN status SET NOT NULL;

-- Indexes for approval workflow
CREATE INDEX idx_domains_status ON domains(status);
CREATE INDEX idx_domains_pending_review ON domains(created_at DESC) WHERE status = 'pending_review';
CREATE INDEX idx_domains_approved_active ON domains(domain_url) WHERE status = 'approved';
CREATE INDEX idx_domains_submitted_by ON domains(submitted_by);
CREATE INDEX idx_domains_trusted ON domains(is_trusted_domain) WHERE is_trusted_domain = true;

-- Update comments
COMMENT ON COLUMN domains.status IS 'Approval state: pending_review, approved, rejected, suspended';
COMMENT ON COLUMN domains.max_crawl_depth IS 'Maximum depth for crawler (prevents runaway crawling). 0=homepage only, 3=recommended';
COMMENT ON COLUMN domains.crawl_rate_limit_seconds IS 'Seconds between requests (be polite to servers)';
COMMENT ON COLUMN domains.is_trusted_domain IS 'If true, individual URLs from this domain bypass review';
COMMENT ON COLUMN domains.submission_context IS 'Why submitter thinks this domain has resources: "State housing authority website"';

-- Drop domain_scrape_urls table (no longer needed - crawler discovers pages automatically)
DROP TABLE IF EXISTS domain_scrape_urls CASCADE;

COMMENT ON TABLE domains IS 'Approved domains for crawling. Crawler automatically discovers pages within each domain.';
