-- Proposal comments: admin feedback on sync proposals for AI-driven refinement.
-- Part of the AI consultant pipeline: admin comments → AI revises → admin approves/edits.

CREATE TABLE proposal_comments (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    proposal_id UUID NOT NULL REFERENCES sync_proposals(id) ON DELETE CASCADE,
    author_id UUID NOT NULL REFERENCES members(id),
    content TEXT NOT NULL,
    revision_number INTEGER NOT NULL DEFAULT 0,
    ai_revised BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_proposal_comments_proposal ON proposal_comments(proposal_id);
CREATE INDEX idx_proposal_comments_author ON proposal_comments(author_id);
