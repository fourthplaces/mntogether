-- Migration 001: Normalize RecallSignals from JSONB to relational table
-- This enables fast relational queries like "find all high-confidence entities across sites"

-- Create enum for signal types
DO $$ BEGIN
    CREATE TYPE signal_type AS ENUM ('cta', 'offer', 'ask', 'entity');
EXCEPTION
    WHEN duplicate_object THEN NULL;
END $$;

-- Create normalized signals table
CREATE TABLE IF NOT EXISTS extraction_signals (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    -- Foreign key to summary (uses URL as key since that's the PK in extraction_summaries)
    summary_url TEXT NOT NULL REFERENCES extraction_summaries(url) ON DELETE CASCADE,
    -- Signal classification
    signal_type signal_type NOT NULL,
    value TEXT NOT NULL,
    -- Entity subtype (for entity signals: 'organization', 'person', 'location', 'date', 'contact', 'phone')
    entity_type TEXT,
    -- Production-hardening columns (Gemini feedback #3)
    confidence FLOAT DEFAULT 1.0 CHECK (confidence >= 0.0 AND confidence <= 1.0),
    context_snippet TEXT,           -- Supporting text showing where signal was found
    group_id UUID,                  -- Group related signals (e.g., same CTA across pages)
    tags TEXT[],                    -- Flexible categorization: ['urgent', 'seasonal', 'verified']
    metadata JSONB DEFAULT '{}',    -- Extension point for domain-specific attributes
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Primary lookup: filter by type + search by value
CREATE INDEX IF NOT EXISTS idx_extraction_signals_type_value
    ON extraction_signals(signal_type, value);

-- Confidence-based filtering (show high-confidence signals first)
CREATE INDEX IF NOT EXISTS idx_extraction_signals_confidence
    ON extraction_signals(confidence DESC);

-- Group aggregation queries
CREATE INDEX IF NOT EXISTS idx_extraction_signals_group
    ON extraction_signals(group_id) WHERE group_id IS NOT NULL;

-- Tag-based filtering (GIN for array containment)
CREATE INDEX IF NOT EXISTS idx_extraction_signals_tags
    ON extraction_signals USING GIN (tags) WHERE tags IS NOT NULL;

-- Summary URL lookup for cascade operations
CREATE INDEX IF NOT EXISTS idx_extraction_signals_summary_url
    ON extraction_signals(summary_url);

-- Trigger function to auto-populate signals from JSONB when summary inserted/updated
CREATE OR REPLACE FUNCTION populate_extraction_signals()
RETURNS TRIGGER AS $$
DECLARE
    signals_json JSONB;
    cta TEXT;
    offer TEXT;
    ask TEXT;
    entity TEXT;
BEGIN
    -- Get signals from the summary (JSONB column)
    signals_json := NEW.signals;

    -- Clear existing signals for this summary
    DELETE FROM extraction_signals WHERE summary_url = NEW.url;

    -- Insert CTAs
    IF signals_json ? 'calls_to_action' THEN
        FOR cta IN SELECT jsonb_array_elements_text(signals_json->'calls_to_action')
        LOOP
            INSERT INTO extraction_signals (summary_url, signal_type, value)
            VALUES (NEW.url, 'cta', cta);
        END LOOP;
    END IF;

    -- Insert Offers
    IF signals_json ? 'offers' THEN
        FOR offer IN SELECT jsonb_array_elements_text(signals_json->'offers')
        LOOP
            INSERT INTO extraction_signals (summary_url, signal_type, value)
            VALUES (NEW.url, 'offer', offer);
        END LOOP;
    END IF;

    -- Insert Asks
    IF signals_json ? 'asks' THEN
        FOR ask IN SELECT jsonb_array_elements_text(signals_json->'asks')
        LOOP
            INSERT INTO extraction_signals (summary_url, signal_type, value)
            VALUES (NEW.url, 'ask', ask);
        END LOOP;
    END IF;

    -- Insert Entities (with type detection heuristics)
    IF signals_json ? 'entities' THEN
        FOR entity IN SELECT jsonb_array_elements_text(signals_json->'entities')
        LOOP
            INSERT INTO extraction_signals (summary_url, signal_type, value, entity_type)
            VALUES (
                NEW.url,
                'entity',
                entity,
                CASE
                    WHEN entity ~* '@|email' THEN 'contact'
                    WHEN entity ~* '^\d{3}[-.]?\d{3}[-.]?\d{4}$' THEN 'phone'
                    WHEN entity ~* '\d{4}' THEN 'date'
                    ELSE NULL
                END
            );
        END LOOP;
    END IF;

    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Apply trigger on insert/update of signals column
DROP TRIGGER IF EXISTS trg_populate_signals ON extraction_summaries;
CREATE TRIGGER trg_populate_signals
    AFTER INSERT OR UPDATE OF signals ON extraction_summaries
    FOR EACH ROW
    EXECUTE FUNCTION populate_extraction_signals();

-- Backfill existing summaries (if any)
-- CTAs
INSERT INTO extraction_signals (summary_url, signal_type, value)
SELECT s.url, 'cta'::signal_type, jsonb_array_elements_text(s.signals->'calls_to_action')
FROM extraction_summaries s
WHERE s.signals ? 'calls_to_action'
  AND jsonb_array_length(s.signals->'calls_to_action') > 0
ON CONFLICT DO NOTHING;

-- Offers
INSERT INTO extraction_signals (summary_url, signal_type, value)
SELECT s.url, 'offer'::signal_type, jsonb_array_elements_text(s.signals->'offers')
FROM extraction_summaries s
WHERE s.signals ? 'offers'
  AND jsonb_array_length(s.signals->'offers') > 0
ON CONFLICT DO NOTHING;

-- Asks
INSERT INTO extraction_signals (summary_url, signal_type, value)
SELECT s.url, 'ask'::signal_type, jsonb_array_elements_text(s.signals->'asks')
FROM extraction_summaries s
WHERE s.signals ? 'asks'
  AND jsonb_array_length(s.signals->'asks') > 0
ON CONFLICT DO NOTHING;

-- Entities
INSERT INTO extraction_signals (summary_url, signal_type, value, entity_type)
SELECT
    s.url,
    'entity'::signal_type,
    e.entity,
    CASE
        WHEN e.entity ~* '@|email' THEN 'contact'
        WHEN e.entity ~* '^\d{3}[-.]?\d{3}[-.]?\d{4}$' THEN 'phone'
        WHEN e.entity ~* '\d{4}' THEN 'date'
        ELSE NULL
    END
FROM extraction_summaries s,
     LATERAL jsonb_array_elements_text(s.signals->'entities') AS e(entity)
WHERE s.signals ? 'entities'
  AND jsonb_array_length(s.signals->'entities') > 0
ON CONFLICT DO NOTHING;
