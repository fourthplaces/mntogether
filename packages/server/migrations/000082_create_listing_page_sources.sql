-- Junction table linking listings to their source page snapshots
-- A listing may be synthesized from multiple pages

CREATE TABLE listing_page_sources (
    listing_id UUID NOT NULL REFERENCES listings(id) ON DELETE CASCADE,
    page_snapshot_id UUID NOT NULL REFERENCES page_snapshots(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (listing_id, page_snapshot_id)
);

-- Index for finding all sources for a listing
CREATE INDEX idx_listing_page_sources_listing ON listing_page_sources(listing_id);

-- Index for finding all listings from a page
CREATE INDEX idx_listing_page_sources_snapshot ON listing_page_sources(page_snapshot_id);
