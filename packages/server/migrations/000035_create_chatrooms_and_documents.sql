-- Create anonymous chatrooms and referral documents system
-- NO authentication required - completely public

-- Step 1: Create chatrooms (anonymous conversations)
CREATE TABLE chatrooms (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  language TEXT DEFAULT 'en' REFERENCES active_languages(language_code),
  created_at TIMESTAMPTZ DEFAULT NOW(),
  last_activity_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_chatrooms_activity ON chatrooms(last_activity_at DESC);

COMMENT ON TABLE chatrooms IS 'Anonymous chat sessions - no auth required';
COMMENT ON COLUMN chatrooms.language IS 'Language for this conversation';

-- Step 2: Create messages
CREATE TABLE messages (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  chatroom_id UUID NOT NULL REFERENCES chatrooms(id) ON DELETE CASCADE,
  role TEXT NOT NULL CHECK (role IN ('user', 'assistant')),
  content TEXT NOT NULL,
  created_at TIMESTAMPTZ DEFAULT NOW(),
  sequence_number INT NOT NULL
);

CREATE INDEX idx_messages_chatroom ON messages(chatroom_id, sequence_number);

COMMENT ON TABLE messages IS 'Messages in chatroom (user and AI assistant)';
COMMENT ON COLUMN messages.sequence_number IS 'Message order in conversation';

-- Step 3: Create referral documents
CREATE TABLE referral_documents (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  chatroom_id UUID REFERENCES chatrooms(id),

  -- Source language (language it was created in)
  source_language TEXT NOT NULL REFERENCES active_languages(language_code),

  -- Content (Markdown + JSX-like components)
  content TEXT NOT NULL,

  -- Shareable link
  slug TEXT UNIQUE NOT NULL,
  title TEXT,
  status TEXT DEFAULT 'draft' CHECK (status IN ('draft', 'published', 'archived')),

  -- Edit capability (no auth - just know the secret token)
  edit_token TEXT UNIQUE,

  -- Analytics
  view_count INT DEFAULT 0,
  last_viewed_at TIMESTAMPTZ,

  created_at TIMESTAMPTZ DEFAULT NOW(),
  updated_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_documents_slug ON referral_documents(slug);
CREATE INDEX idx_documents_chatroom ON referral_documents(chatroom_id);
CREATE INDEX idx_documents_language ON referral_documents(source_language);
CREATE INDEX idx_documents_status ON referral_documents(status);

COMMENT ON TABLE referral_documents IS 'Generated referral documents (markdown + components) - completely public, no auth';
COMMENT ON COLUMN referral_documents.content IS 'Markdown with JSX-like components: <Listing id="..." />, <Map>, <Contact>';
COMMENT ON COLUMN referral_documents.edit_token IS 'Secret token for editing (no auth required)';
COMMENT ON COLUMN referral_documents.slug IS 'Human-readable URL slug (e.g., warm-mountain-7423)';

-- Step 4: Create referral document translations
CREATE TABLE referral_document_translations (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  document_id UUID NOT NULL REFERENCES referral_documents(id) ON DELETE CASCADE,
  language_code TEXT NOT NULL REFERENCES active_languages(language_code),
  content TEXT NOT NULL,
  title TEXT,
  translated_at TIMESTAMPTZ DEFAULT NOW(),
  translation_model TEXT DEFAULT 'gpt-4o',
  UNIQUE(document_id, language_code)
);

CREATE INDEX idx_document_translations_document ON referral_document_translations(document_id);
CREATE INDEX idx_document_translations_language ON referral_document_translations(language_code);

COMMENT ON TABLE referral_document_translations IS 'Translated versions of referral documents';

-- Step 5: Create document references (for staleness detection)
CREATE TABLE document_references (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  document_id UUID NOT NULL REFERENCES referral_documents(id) ON DELETE CASCADE,
  reference_kind TEXT NOT NULL CHECK (reference_kind IN ('listing', 'organization', 'contact')),
  reference_id TEXT NOT NULL,
  referenced_at TIMESTAMPTZ DEFAULT NOW(),
  display_order INT DEFAULT 0,
  UNIQUE(document_id, reference_kind, reference_id)
);

CREATE INDEX idx_document_refs_document ON document_references(document_id);
CREATE INDEX idx_document_refs_kind_id ON document_references(reference_kind, reference_id);

COMMENT ON TABLE document_references IS 'Tracks entities referenced in documents for staleness detection';
COMMENT ON COLUMN document_references.reference_kind IS 'Type of entity: listing, organization, contact';
COMMENT ON COLUMN document_references.reference_id IS 'UUID of referenced entity';
