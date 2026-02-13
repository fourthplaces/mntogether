-- Fix: pg_notify has an 8000-byte payload limit.
-- The original trigger included the full event payload, causing
-- "payload string too long" errors for large events (e.g. 13+ extracted posts).
-- The notification is only a wake-up signal — strip the payload.

CREATE OR REPLACE FUNCTION seesaw_notify_saga() RETURNS trigger AS $$
BEGIN
    PERFORM pg_notify(
        'seesaw_saga_' || NEW.correlation_id::text,
        json_build_object(
            'event_id', NEW.event_id,
            'correlation_id', NEW.correlation_id,
            'event_type', NEW.event_type
        )::text
    );
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;
