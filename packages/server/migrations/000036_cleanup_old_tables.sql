-- Cleanup: Drop old tables that have been replaced by refactoring
-- Safe because: project not launched yet, data already migrated

-- Step 1: Drop organization_sources (replaced by domains + organizations)
DROP TABLE IF EXISTS organization_sources CASCADE;

COMMENT ON TABLE domains IS 'Replaced organization_sources - now separates domains (scraping) from organizations (entities)';

-- Step 2: Drop auth-related tables (we're going fully anonymous)
DROP TABLE IF EXISTS volunteers CASCADE;
DROP TABLE IF EXISTS volunteer_skills CASCADE;
DROP TABLE IF EXISTS volunteer_availabilities CASCADE;

COMMENT ON TABLE chatrooms IS 'No auth required - anonymous by design';

-- Step 3: Drop matching tables if they exist (focusing on directory + AI assist instead)
DROP TABLE IF EXISTS matches CASCADE;
DROP TABLE IF EXISTS match_feedback CASCADE;

-- Step 4: Drop old middleware auth table
DROP TABLE IF EXISTS clerk_sessions CASCADE;

-- Verification of cleanup
DO $$
BEGIN
  RAISE NOTICE 'Cleanup complete. Remaining core tables:';
  RAISE NOTICE '  - domains (scraping config)';
  RAISE NOTICE '  - organizations (entities)';
  RAISE NOTICE '  - listings (base + service/opportunity/business tables)';
  RAISE NOTICE '  - tags + taggables (flexible metadata)';
  RAISE NOTICE '  - active_languages + translations';
  RAISE NOTICE '  - chatrooms + messages + referral_documents';
END $$;
