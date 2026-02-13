-- Notes: standalone annotations with severity levels, source tracking, and polymorphic linking
CREATE TABLE notes (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    content TEXT NOT NULL,
    severity TEXT NOT NULL DEFAULT 'info',
    source_url TEXT,
    source_id UUID,
    source_type TEXT,
    is_public BOOLEAN NOT NULL DEFAULT false,
    created_by TEXT NOT NULL DEFAULT 'system',
    expired_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_notes_severity ON notes(severity);
CREATE INDEX idx_notes_expired_at ON notes(expired_at) WHERE expired_at IS NULL;
CREATE INDEX idx_notes_source ON notes(source_type, source_id) WHERE source_id IS NOT NULL;
CREATE INDEX idx_notes_is_public ON notes(is_public) WHERE is_public = true;

-- Noteables: polymorphic join table linking notes to any entity
CREATE TABLE noteables (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    note_id UUID NOT NULL REFERENCES notes(id) ON DELETE CASCADE,
    noteable_type TEXT NOT NULL,
    noteable_id UUID NOT NULL,
    added_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(note_id, noteable_type, noteable_id)
);

CREATE INDEX idx_noteables_entity ON noteables(noteable_type, noteable_id);
CREATE INDEX idx_noteables_note ON noteables(note_id);
