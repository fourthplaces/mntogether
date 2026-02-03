-- Migration: Drop unused crawler tables from migration 000025
--
-- These tables were created for a schema-based extraction system that was
-- never fully implemented. The extraction library now handles all AI extraction,
-- storage, and embedding via extraction_* tables.
--
-- Tables being dropped:
-- - schemas: External schema registry (never used)
-- - detections: AI detection records (never used)
-- - extractions: Schema-based extractions (never used - NOT extraction_* tables!)
-- - field_provenance: Field tracing (never used)
-- - relationships: Graph edges (never used)
--
-- NOT dropping (still used by existing crawl code):
-- - page_snapshots: Used by crawl_website action
-- - page_summaries: Used by summarize effect
-- - page_extractions: Used by agentic extraction
--
-- These will be dropped in a future migration once crawling code is fully
-- migrated to use the extraction library's Ingestor pattern.

-- Drop in dependency order (children first, then parents)
DROP TABLE IF EXISTS field_provenance CASCADE;
DROP TABLE IF EXISTS relationships CASCADE;
DROP TABLE IF EXISTS detections CASCADE;
DROP TABLE IF EXISTS extractions CASCADE;
DROP TABLE IF EXISTS schemas CASCADE;
