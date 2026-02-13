-- Seesaw 0.10.2: Batch/Join support tables and columns

ALTER TABLE seesaw_events ADD COLUMN IF NOT EXISTS batch_id UUID;
ALTER TABLE seesaw_events ADD COLUMN IF NOT EXISTS batch_index INTEGER;
ALTER TABLE seesaw_events ADD COLUMN IF NOT EXISTS batch_size INTEGER;

ALTER TABLE seesaw_effect_executions ADD COLUMN IF NOT EXISTS batch_id UUID;
ALTER TABLE seesaw_effect_executions ADD COLUMN IF NOT EXISTS batch_index INTEGER;
ALTER TABLE seesaw_effect_executions ADD COLUMN IF NOT EXISTS batch_size INTEGER;

CREATE TABLE IF NOT EXISTS seesaw_join_entries (
    join_effect_id VARCHAR NOT NULL,
    correlation_id UUID NOT NULL,
    source_event_id UUID NOT NULL,
    source_event_type VARCHAR NOT NULL,
    source_payload JSONB NOT NULL,
    source_created_at TIMESTAMPTZ NOT NULL,
    batch_id UUID NOT NULL,
    batch_index INTEGER NOT NULL,
    batch_size INTEGER NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (join_effect_id, correlation_id, source_event_id)
);

CREATE TABLE IF NOT EXISTS seesaw_join_windows (
    join_effect_id VARCHAR NOT NULL,
    correlation_id UUID NOT NULL,
    mode VARCHAR NOT NULL DEFAULT 'same_batch',
    batch_id UUID NOT NULL,
    target_count INTEGER NOT NULL,
    status VARCHAR NOT NULL DEFAULT 'open',
    sealed_at TIMESTAMPTZ,
    processing_started_at TIMESTAMPTZ,
    completed_at TIMESTAMPTZ,
    last_error VARCHAR,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (join_effect_id, correlation_id, batch_id)
);
