-- Media library: stores metadata for uploaded files (images, documents).
-- Actual files live in S3-compatible storage (MinIO for dev, AWS S3/R2 for prod).

CREATE TABLE media (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    filename TEXT NOT NULL,
    content_type TEXT NOT NULL,
    size_bytes BIGINT NOT NULL,
    storage_key TEXT NOT NULL UNIQUE,
    url TEXT NOT NULL,
    alt_text TEXT,
    width INT,
    height INT,
    uploaded_by UUID REFERENCES members(id) ON DELETE SET NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_media_created_at ON media(created_at DESC);
CREATE INDEX idx_media_content_type ON media(content_type);
CREATE INDEX idx_media_uploaded_by ON media(uploaded_by);
