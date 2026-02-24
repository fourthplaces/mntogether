-- Fix: Change confidence_score from NUMERIC to DOUBLE PRECISION
-- The Rust model uses f64 which maps to FLOAT8/DOUBLE PRECISION

ALTER TABLE website_assessments
    ALTER COLUMN confidence_score TYPE DOUBLE PRECISION;
