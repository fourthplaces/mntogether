-- Create multi-language system (dynamic language support)

-- Step 1: Create active_languages table
CREATE TABLE active_languages (
  language_code TEXT PRIMARY KEY,
  language_name TEXT NOT NULL,
  native_name TEXT NOT NULL,
  enabled BOOL DEFAULT true,
  added_at TIMESTAMPTZ DEFAULT NOW()
);

COMMENT ON TABLE active_languages IS 'Dynamic language system - add languages without code changes';
COMMENT ON COLUMN active_languages.language_code IS 'ISO 639-1 code (en, es, so, etc.)';

-- Step 2: Seed MVP languages (English, Spanish, Somali)
INSERT INTO active_languages (language_code, language_name, native_name) VALUES
  ('en', 'English', 'English'),
  ('es', 'Spanish', 'Español'),
  ('so', 'Somali', 'Soomaali');

-- Step 3: Create listing translations table
CREATE TABLE listing_translations (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  listing_id UUID NOT NULL REFERENCES listings(id) ON DELETE CASCADE,
  language_code TEXT NOT NULL REFERENCES active_languages(language_code),
  title TEXT NOT NULL,
  description TEXT NOT NULL,
  tldr TEXT,
  translated_at TIMESTAMPTZ DEFAULT NOW(),
  translation_model TEXT DEFAULT 'gpt-4o',
  UNIQUE(listing_id, language_code)
);

CREATE INDEX idx_listing_translations_listing ON listing_translations(listing_id);
CREATE INDEX idx_listing_translations_language ON listing_translations(language_code);

COMMENT ON TABLE listing_translations IS 'Cached translations for listings (via seesaw TranslateRequest command)';
COMMENT ON COLUMN listing_translations.translation_model IS 'AI model used for translation';

-- Step 4: Auto-translate existing listings to Spanish and Somali
-- (This will be done via seesaw BatchTranslate command after migration)
-- For now, just ensure the schema is ready

COMMENT ON TABLE listing_translations IS 'Translations cached in DB. To add new language: INSERT INTO active_languages, then trigger BatchTranslate seesaw command.';
