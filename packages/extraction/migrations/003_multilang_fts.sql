-- Migration 003: Multi-language full-text search support
-- Adds language detection and appropriate FTS configuration per page

-- Add language column to pages table
ALTER TABLE extraction_pages
    ADD COLUMN IF NOT EXISTS language TEXT DEFAULT 'english';

-- Create a generated column for tsvector that respects the page's language
-- Note: PostgreSQL's to_tsvector accepts regconfig as first argument
-- We need a function to safely cast language string to regconfig

CREATE OR REPLACE FUNCTION safe_to_tsvector(lang TEXT, content TEXT)
RETURNS tsvector AS $$
DECLARE
    cfg regconfig;
BEGIN
    -- Try to use the specified language, fall back to 'english' if invalid
    BEGIN
        cfg := lang::regconfig;
    EXCEPTION WHEN OTHERS THEN
        cfg := 'english'::regconfig;
    END;

    RETURN to_tsvector(cfg, coalesce(content, ''));
END;
$$ LANGUAGE plpgsql IMMUTABLE;

-- Add content_tsvector as a stored generated column
-- Note: This uses COALESCE to include title in the searchable content
DO $$
BEGIN
    -- Check if column exists
    IF NOT EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_name = 'extraction_pages' AND column_name = 'content_tsvector'
    ) THEN
        -- PostgreSQL 12+ supports generated columns
        -- We use a trigger instead for broader compatibility and dynamic language support
        ALTER TABLE extraction_pages ADD COLUMN content_tsvector tsvector;
    END IF;
END $$;

-- Create trigger function to maintain tsvector
CREATE OR REPLACE FUNCTION update_page_tsvector()
RETURNS TRIGGER AS $$
DECLARE
    cfg regconfig;
BEGIN
    -- Safely get regconfig, defaulting to english
    BEGIN
        cfg := COALESCE(NEW.language, 'english')::regconfig;
    EXCEPTION WHEN OTHERS THEN
        cfg := 'english'::regconfig;
    END;

    -- Combine title and content for search
    NEW.content_tsvector := to_tsvector(cfg,
        coalesce(NEW.title, '') || ' ' || coalesce(NEW.content, '')
    );

    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Apply trigger
DROP TRIGGER IF EXISTS trg_update_page_tsvector ON extraction_pages;
CREATE TRIGGER trg_update_page_tsvector
    BEFORE INSERT OR UPDATE OF title, content, language ON extraction_pages
    FOR EACH ROW
    EXECUTE FUNCTION update_page_tsvector();

-- Create GIN index for fast FTS queries
DROP INDEX IF EXISTS idx_extraction_pages_content_tsvector;
CREATE INDEX idx_extraction_pages_fts ON extraction_pages USING GIN (content_tsvector);

-- Backfill existing pages with tsvector
UPDATE extraction_pages SET
    content_tsvector = to_tsvector(
        COALESCE(language, 'english')::regconfig,
        coalesce(title, '') || ' ' || coalesce(content, '')
    )
WHERE content_tsvector IS NULL;

-- Supported languages (for reference - PostgreSQL built-in):
-- 'arabic', 'armenian', 'basque', 'catalan', 'danish', 'dutch', 'english',
-- 'finnish', 'french', 'german', 'greek', 'hindi', 'hungarian', 'indonesian',
-- 'irish', 'italian', 'lithuanian', 'nepali', 'norwegian', 'portuguese',
-- 'romanian', 'russian', 'serbian', 'spanish', 'swedish', 'tamil', 'turkish', 'yiddish'
