-- Create intelligent crawler tables
-- These tables support the intelligent web crawling library with PostgreSQL storage

-- Page snapshots: immutable snapshots of crawled pages
CREATE TABLE page_snapshots (
    id UUID PRIMARY KEY,
    url TEXT NOT NULL,
    content_hash BYTEA NOT NULL,
    html TEXT NOT NULL,
    markdown TEXT,
    fetched_via TEXT NOT NULL,
    metadata JSONB NOT NULL DEFAULT '{}',
    crawled_at TIMESTAMPTZ NOT NULL,
    UNIQUE(url, content_hash)
);

CREATE INDEX idx_page_snapshots_url ON page_snapshots(url);
CREATE INDEX idx_page_snapshots_content_hash ON page_snapshots(content_hash);
CREATE INDEX idx_page_snapshots_crawled_at ON page_snapshots(crawled_at DESC);

-- Schemas: external schema registry for structured extraction
CREATE TABLE schemas (
    id UUID PRIMARY KEY,
    name TEXT NOT NULL,
    version INTEGER NOT NULL,
    json_schema JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL,
    UNIQUE(name, version)
);

CREATE INDEX idx_schemas_name ON schemas(name);

-- Detections: AI/heuristic detection that page contains relevant info
CREATE TABLE detections (
    id UUID PRIMARY KEY,
    page_snapshot_id UUID NOT NULL REFERENCES page_snapshots(id) ON DELETE CASCADE,
    kind TEXT NOT NULL,
    confidence_overall REAL NOT NULL,
    confidence_heuristic REAL,
    confidence_ai REAL,
    origin JSONB NOT NULL,
    evidence JSONB NOT NULL,
    detected_at TIMESTAMPTZ NOT NULL
);

CREATE INDEX idx_detections_page_snapshot_id ON detections(page_snapshot_id);
CREATE INDEX idx_detections_kind ON detections(kind);
CREATE INDEX idx_detections_confidence ON detections(confidence_overall DESC);

-- Extractions: structured data extracted from a page
CREATE TABLE extractions (
    id UUID PRIMARY KEY,
    fingerprint BYTEA NOT NULL,
    page_snapshot_id UUID NOT NULL REFERENCES page_snapshots(id) ON DELETE CASCADE,
    schema_id UUID NOT NULL REFERENCES schemas(id) ON DELETE CASCADE,
    schema_version INTEGER NOT NULL,
    data JSONB NOT NULL,
    confidence_overall REAL NOT NULL,
    confidence_heuristic REAL,
    confidence_ai REAL,
    origin JSONB NOT NULL,
    extracted_at TIMESTAMPTZ NOT NULL,
    UNIQUE(fingerprint, schema_id, schema_version)
);

CREATE INDEX idx_extractions_page_snapshot_id ON extractions(page_snapshot_id);
CREATE INDEX idx_extractions_schema_id ON extractions(schema_id);
CREATE INDEX idx_extractions_fingerprint ON extractions(fingerprint);
CREATE INDEX idx_extractions_extracted_at ON extractions(extracted_at DESC);

-- Field provenance: traces each field to its location in the page
CREATE TABLE field_provenance (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    extraction_id UUID NOT NULL REFERENCES extractions(id) ON DELETE CASCADE,
    field_path TEXT NOT NULL,
    source_location TEXT NOT NULL,
    extraction_method TEXT NOT NULL
);

CREATE INDEX idx_field_provenance_extraction_id ON field_provenance(extraction_id);

-- Relationships: first-class graph edges between extractions
CREATE TABLE relationships (
    id UUID PRIMARY KEY,
    from_extraction_id UUID NOT NULL REFERENCES extractions(id) ON DELETE CASCADE,
    to_extraction_id UUID NOT NULL REFERENCES extractions(id) ON DELETE CASCADE,
    kind TEXT NOT NULL,
    confidence_overall REAL NOT NULL,
    confidence_heuristic REAL,
    confidence_ai REAL,
    origin JSONB NOT NULL,
    metadata JSONB NOT NULL DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL,
    UNIQUE(from_extraction_id, to_extraction_id, kind)
);

CREATE INDEX idx_relationships_from_extraction_id ON relationships(from_extraction_id);
CREATE INDEX idx_relationships_to_extraction_id ON relationships(to_extraction_id);
CREATE INDEX idx_relationships_kind ON relationships(kind);
