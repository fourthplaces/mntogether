-- Add fingerprint for better change detection (normalized content for matching)
-- Add confidence for admin triage (AI extraction confidence)

ALTER TABLE organization_needs
    ADD COLUMN fingerprint TEXT,
    ADD COLUMN extraction_confidence TEXT CHECK (extraction_confidence IN ('high', 'medium', 'low'));

-- Index for fingerprint lookup (replaces brittle title matching)
CREATE INDEX idx_organization_needs_fingerprint
    ON organization_needs(fingerprint)
    WHERE fingerprint IS NOT NULL;

-- Comment explaining fingerprint
COMMENT ON COLUMN organization_needs.fingerprint IS
'Normalized content fingerprint: lowercase, no punctuation, first 300 chars of description. Used to detect content changes more reliably than title matching.';

COMMENT ON COLUMN organization_needs.extraction_confidence IS
'AI extraction confidence (high/medium/low). Used for admin triage, NOT decision-making. Sort low confidence first in approval queue.';
