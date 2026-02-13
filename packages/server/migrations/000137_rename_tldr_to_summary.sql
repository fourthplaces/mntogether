-- Rename tldr → summary
-- Uses DO blocks to be idempotent (safe to re-run if partially applied)
DO $$ BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'posts' AND column_name = 'tldr') THEN
        ALTER TABLE posts RENAME COLUMN tldr TO summary;
    END IF;
END $$;

DO $$ BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'listing_translations' AND column_name = 'tldr') THEN
        ALTER TABLE listing_translations RENAME COLUMN tldr TO summary;
    END IF;
END $$;
