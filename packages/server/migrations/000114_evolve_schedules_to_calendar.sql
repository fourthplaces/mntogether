-- Evolve schedules table to support calendar events (one-off, recurring, all-day)
-- alongside existing operating-hours use case.
-- Occurrences are computed on the fly via the rrule crate — no projection table needed.

-- Add calendar fields to schedules
ALTER TABLE schedules ADD COLUMN dtstart TIMESTAMPTZ;
ALTER TABLE schedules ADD COLUMN dtend TIMESTAMPTZ;
ALTER TABLE schedules ADD COLUMN rrule TEXT;
ALTER TABLE schedules ADD COLUMN exdates TEXT;
ALTER TABLE schedules ADD COLUMN is_all_day BOOLEAN NOT NULL DEFAULT false;
ALTER TABLE schedules ADD COLUMN duration_minutes INTEGER;
ALTER TABLE schedules ADD COLUMN updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW();

-- day_of_week is now optional (rrule handles recurrence)
ALTER TABLE schedules ALTER COLUMN day_of_week DROP NOT NULL;
ALTER TABLE schedules DROP CONSTRAINT IF EXISTS schedules_day_of_week_check;
ALTER TABLE schedules ADD CONSTRAINT schedules_day_of_week_check CHECK (day_of_week IS NULL OR day_of_week BETWEEN 0 AND 6);

-- Backfill rrule for any existing day_of_week rows
UPDATE schedules SET rrule = CASE day_of_week
    WHEN 0 THEN 'FREQ=WEEKLY;BYDAY=SU'
    WHEN 1 THEN 'FREQ=WEEKLY;BYDAY=MO'
    WHEN 2 THEN 'FREQ=WEEKLY;BYDAY=TU'
    WHEN 3 THEN 'FREQ=WEEKLY;BYDAY=WE'
    WHEN 4 THEN 'FREQ=WEEKLY;BYDAY=TH'
    WHEN 5 THEN 'FREQ=WEEKLY;BYDAY=FR'
    WHEN 6 THEN 'FREQ=WEEKLY;BYDAY=SA'
END WHERE rrule IS NULL AND day_of_week IS NOT NULL;

COMMENT ON COLUMN schedules.rrule IS 'RFC 5545 recurrence rule string (e.g. FREQ=WEEKLY;BYDAY=MO)';
COMMENT ON COLUMN schedules.exdates IS 'Comma-separated ISO dates for exception dates';
COMMENT ON COLUMN schedules.dtstart IS 'Start datetime for one-off or recurring events';
COMMENT ON COLUMN schedules.dtend IS 'End datetime for one-off events';
COMMENT ON COLUMN schedules.is_all_day IS 'Whether this is an all-day event';
COMMENT ON COLUMN schedules.duration_minutes IS 'Duration in minutes (alternative to dtend for recurring)';
