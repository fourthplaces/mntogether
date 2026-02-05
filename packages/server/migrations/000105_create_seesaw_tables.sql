-- Seesaw queue-backed engine tables
-- Used by seesaw-postgres PostgresStore for the QueueEngine

-- seesaw_events: Event queue (partitioned by created_at for retention)
CREATE TABLE seesaw_events (
    id BIGSERIAL,
    event_id UUID NOT NULL,
    parent_id UUID,
    correlation_id UUID NOT NULL,
    event_type TEXT NOT NULL,
    payload JSONB NOT NULL DEFAULT '{}',
    hops INTEGER NOT NULL DEFAULT 0,
    retry_count INTEGER NOT NULL DEFAULT 0,
    locked_until TIMESTAMPTZ,
    processed_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (id, created_at)
) PARTITION BY RANGE (created_at);

-- Create default partition for all data
CREATE TABLE seesaw_events_default PARTITION OF seesaw_events DEFAULT;

CREATE INDEX idx_seesaw_events_poll ON seesaw_events (correlation_id, created_at, id)
    WHERE processed_at IS NULL;
CREATE UNIQUE INDEX idx_seesaw_events_idempotency ON seesaw_events (event_id, created_at);

-- seesaw_processed: Idempotency guard (dedup by event_id)
CREATE TABLE seesaw_processed (
    event_id UUID PRIMARY KEY,
    correlation_id UUID NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- seesaw_state: Saga/workflow state (optimistic locking)
CREATE TABLE seesaw_state (
    correlation_id UUID PRIMARY KEY,
    state JSONB NOT NULL DEFAULT '{}',
    version INTEGER NOT NULL DEFAULT 0,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- seesaw_effect_executions: Effect execution tracking
CREATE TABLE seesaw_effect_executions (
    event_id UUID NOT NULL,
    effect_id TEXT NOT NULL,
    correlation_id UUID NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending',
    event_type TEXT NOT NULL,
    event_payload JSONB NOT NULL DEFAULT '{}',
    parent_event_id UUID,
    execute_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    timeout_seconds INTEGER NOT NULL DEFAULT 30,
    max_attempts INTEGER NOT NULL DEFAULT 3,
    priority INTEGER NOT NULL DEFAULT 0,
    attempts INTEGER NOT NULL DEFAULT 0,
    result JSONB,
    error TEXT,
    claimed_at TIMESTAMPTZ,
    last_attempted_at TIMESTAMPTZ,
    completed_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (event_id, effect_id)
);

CREATE INDEX idx_seesaw_effects_poll ON seesaw_effect_executions (priority, execute_at, event_id, effect_id)
    WHERE status = 'pending';
CREATE INDEX idx_seesaw_effects_saga ON seesaw_effect_executions (correlation_id, status);

-- seesaw_dlq: Dead letter queue for permanently failed effects
CREATE TABLE seesaw_dlq (
    id BIGSERIAL PRIMARY KEY,
    event_id UUID NOT NULL,
    effect_id TEXT NOT NULL,
    correlation_id UUID NOT NULL,
    error TEXT NOT NULL,
    event_type TEXT NOT NULL,
    event_payload JSONB NOT NULL DEFAULT '{}',
    reason TEXT NOT NULL DEFAULT 'max_retries_exceeded',
    attempts INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Notification function for .wait() pattern (LISTEN/NOTIFY)
CREATE OR REPLACE FUNCTION seesaw_notify_saga() RETURNS trigger AS $$
BEGIN
    PERFORM pg_notify(
        'seesaw_saga_' || NEW.correlation_id::text,
        json_build_object(
            'event_id', NEW.event_id,
            'correlation_id', NEW.correlation_id,
            'event_type', NEW.event_type,
            'payload', NEW.payload
        )::text
    );
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER seesaw_events_notify
    AFTER INSERT ON seesaw_events
    FOR EACH ROW EXECUTE FUNCTION seesaw_notify_saga();
