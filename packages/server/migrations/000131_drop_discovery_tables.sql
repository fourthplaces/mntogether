-- Drop discovery domain tables (replaced by agents domain)
-- Order matters: drop tables with foreign keys first

DROP TABLE IF EXISTS discovery_run_results;
DROP TABLE IF EXISTS discovery_runs;
DROP TABLE IF EXISTS discovery_filter_rules;
DROP TABLE IF EXISTS discovery_queries;
