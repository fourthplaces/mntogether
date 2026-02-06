-- Staged AI Proposals: Human-in-the-loop review system
--
-- Generic proposal tables where AI-proposed changes (inserts, updates, deletes, merges)
-- are staged for human review before being applied.

-- sync_batches: Groups proposals from one AI operation
CREATE TABLE sync_batches (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    resource_type TEXT NOT NULL,
    source_id UUID,
    status TEXT NOT NULL DEFAULT 'pending'
        CHECK (status IN ('pending', 'partially_reviewed', 'completed', 'expired')),
    summary TEXT,
    proposal_count INTEGER NOT NULL DEFAULT 0,
    approved_count INTEGER NOT NULL DEFAULT 0,
    rejected_count INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    reviewed_at TIMESTAMPTZ,
    expires_at TIMESTAMPTZ
);

CREATE INDEX idx_sync_batches_resource_source ON sync_batches (resource_type, source_id);
CREATE INDEX idx_sync_batches_status ON sync_batches (status);

-- sync_proposals: Individual proposed operations (polymorphic)
CREATE TABLE sync_proposals (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    batch_id UUID NOT NULL REFERENCES sync_batches(id) ON DELETE CASCADE,
    operation TEXT NOT NULL CHECK (operation IN ('insert', 'update', 'delete', 'merge')),
    status TEXT NOT NULL DEFAULT 'pending'
        CHECK (status IN ('pending', 'approved', 'rejected')),
    entity_type TEXT NOT NULL,
    draft_entity_id UUID,
    target_entity_id UUID,
    reason TEXT,
    reviewed_by UUID REFERENCES members(id),
    reviewed_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_sync_proposals_batch_id ON sync_proposals (batch_id);
CREATE INDEX idx_sync_proposals_status ON sync_proposals (status);
CREATE INDEX idx_sync_proposals_entity ON sync_proposals (entity_type, target_entity_id);

-- sync_proposal_merge_sources: Entities to absorb in MERGE proposals
CREATE TABLE sync_proposal_merge_sources (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    proposal_id UUID NOT NULL REFERENCES sync_proposals(id) ON DELETE CASCADE,
    source_entity_id UUID NOT NULL,
    UNIQUE(proposal_id, source_entity_id)
);

CREATE INDEX idx_sync_proposal_merge_sources_proposal ON sync_proposal_merge_sources (proposal_id);
