-- Lower word similarity threshold for fuzzy search.
-- Default is 0.6 which is too strict for typo-tolerant search.
-- 0.3 catches reasonable misspellings (1-2 char typos in short words).
ALTER DATABASE rooteditorial SET pg_trgm.word_similarity_threshold = 0.3;
