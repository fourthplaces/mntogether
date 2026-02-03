-- Migration 005: Generalize signal_type from ENUM to TEXT
-- This makes the library domain-agnostic. Users define their own signal types.
--
-- Before: signal_type ENUM ('cta', 'offer', 'ask', 'entity')  -- domain-specific
-- After:  signal_type TEXT NOT NULL                           -- user-defined

-- Step 1: Add a temporary TEXT column
ALTER TABLE extraction_signals ADD COLUMN signal_type_new TEXT;

-- Step 2: Copy data from enum to text
UPDATE extraction_signals SET signal_type_new = signal_type::TEXT;

-- Step 3: Drop the old enum column
ALTER TABLE extraction_signals DROP COLUMN signal_type;

-- Step 4: Rename the new column
ALTER TABLE extraction_signals RENAME COLUMN signal_type_new TO signal_type;

-- Step 5: Add NOT NULL constraint
ALTER TABLE extraction_signals ALTER COLUMN signal_type SET NOT NULL;

-- Step 6: Recreate the index (was on enum, now on text)
DROP INDEX IF EXISTS idx_extraction_signals_type_value;
CREATE INDEX idx_extraction_signals_type_value ON extraction_signals(signal_type, value);

-- Step 7: Add index for type-only queries (common pattern)
CREATE INDEX IF NOT EXISTS idx_extraction_signals_type ON extraction_signals(signal_type);

-- Step 8: Drop the enum type (no longer needed)
-- Note: This will fail silently if the type is still referenced elsewhere
DO $$ BEGIN
    DROP TYPE IF EXISTS signal_type;
EXCEPTION
    WHEN dependent_objects_still_exist THEN NULL;
END $$;

-- Step 9: Rename entity_type to subtype for consistency with Rust struct
ALTER TABLE extraction_signals RENAME COLUMN entity_type TO subtype;

-- Step 9b: Add source_id for evidence grounding (every signal traceable to source)
ALTER TABLE extraction_signals ADD COLUMN IF NOT EXISTS source_id UUID;

-- Index for source-based queries (e.g., "all signals from this page")
CREATE INDEX IF NOT EXISTS idx_extraction_signals_source ON extraction_signals(source_id) WHERE source_id IS NOT NULL;

-- Step 10: Update the trigger function to handle generic signal types
-- The trigger now just populates from legacy JSONB without hardcoded types
CREATE OR REPLACE FUNCTION populate_extraction_signals()
RETURNS TRIGGER AS $$
DECLARE
    signals_json JSONB;
    item TEXT;
    signal_category TEXT;
BEGIN
    -- Get signals from the summary (JSONB column)
    signals_json := NEW.signals;

    -- Clear existing signals for this summary
    DELETE FROM extraction_signals WHERE summary_url = NEW.url;

    -- Process each known category from legacy format
    FOREACH signal_category IN ARRAY ARRAY['calls_to_action', 'offers', 'asks', 'entities']
    LOOP
        IF signals_json ? signal_category THEN
            FOR item IN SELECT jsonb_array_elements_text(signals_json->signal_category)
            LOOP
                INSERT INTO extraction_signals (summary_url, signal_type, value, subtype)
                VALUES (
                    NEW.url,
                    CASE signal_category
                        WHEN 'calls_to_action' THEN 'cta'
                        WHEN 'offers' THEN 'offer'
                        WHEN 'asks' THEN 'ask'
                        WHEN 'entities' THEN 'entity'
                    END,
                    item,
                    -- Auto-detect subtype for entities (legacy behavior)
                    CASE
                        WHEN signal_category = 'entities' AND item ~* '@|email' THEN 'contact'
                        WHEN signal_category = 'entities' AND item ~* '^\d{3}[-.]?\d{3}[-.]?\d{4}$' THEN 'phone'
                        WHEN signal_category = 'entities' AND item ~* '\d{4}' THEN 'date'
                        ELSE NULL
                    END
                );
            END LOOP;
        END IF;
    END LOOP;

    -- Also handle new generic format: {"signals": [{"type": "product", "value": "..."}]}
    IF signals_json ? 'signals' AND jsonb_typeof(signals_json->'signals') = 'array' THEN
        INSERT INTO extraction_signals (summary_url, signal_type, value, subtype, confidence, context_snippet, tags)
        SELECT
            NEW.url,
            s->>'type',
            s->>'value',
            s->>'subtype',
            COALESCE((s->>'confidence')::FLOAT, 1.0),
            s->>'context_snippet',
            CASE
                WHEN s ? 'tags' AND jsonb_typeof(s->'tags') = 'array'
                THEN ARRAY(SELECT jsonb_array_elements_text(s->'tags'))
                ELSE NULL
            END
        FROM jsonb_array_elements(signals_json->'signals') AS s
        WHERE s->>'type' IS NOT NULL AND s->>'value' IS NOT NULL;
    END IF;

    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Comment explaining the change
COMMENT ON COLUMN extraction_signals.signal_type IS 'User-defined signal type (e.g., "product", "listing", "cta"). The library is domain-agnostic.';
COMMENT ON COLUMN extraction_signals.subtype IS 'Optional subtype for further classification (e.g., "electronics" for product, "contact" for entity).';
