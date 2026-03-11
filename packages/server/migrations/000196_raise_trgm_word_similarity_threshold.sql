-- Raise word similarity threshold from 0.3 to 0.4.
-- 0.3 was too loose — "housing" falsely matched "Licensing" via shared trigrams.
-- 0.4 rejects those while still catching 1-char typos like "housng" → "Housing".
ALTER DATABASE rooteditorial SET pg_trgm.word_similarity_threshold = 0.4;
