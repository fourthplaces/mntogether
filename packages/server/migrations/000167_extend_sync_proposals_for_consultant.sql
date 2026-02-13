-- Extend sync_proposals with fields for the AI consultant pipeline.
-- consultant_reasoning: the AI's explanation for why this action is recommended
-- revision_count: how many times the proposal has been revised via comment feedback
-- confidence: high/medium/low signal for admin triage
-- source_urls: which crawled pages support this proposed action

ALTER TABLE sync_proposals ADD COLUMN consultant_reasoning TEXT;
ALTER TABLE sync_proposals ADD COLUMN revision_count INTEGER NOT NULL DEFAULT 0;
ALTER TABLE sync_proposals ADD COLUMN confidence TEXT;
ALTER TABLE sync_proposals ADD COLUMN source_urls TEXT[];
