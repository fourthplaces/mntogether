-- Fix: Change score from NUMERIC to DOUBLE PRECISION
-- The Rust model uses f64 which maps to FLOAT8/DOUBLE PRECISION

ALTER TABLE tavily_search_results
    ALTER COLUMN score TYPE DOUBLE PRECISION;
