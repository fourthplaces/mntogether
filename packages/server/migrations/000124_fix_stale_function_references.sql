-- Fix functions whose bodies still reference "listings" after rename to "posts"

-- Fix trigger function: update_page_snapshot_listings_count
CREATE OR REPLACE FUNCTION update_page_snapshot_listings_count()
RETURNS TRIGGER AS $$
BEGIN
  IF NEW.page_snapshot_id IS NOT NULL THEN
    UPDATE page_snapshots
    SET listings_extracted_count = (
      SELECT COUNT(*)
      FROM posts
      WHERE page_snapshot_id = NEW.page_snapshot_id
    )
    WHERE id = NEW.page_snapshot_id;
  END IF;
  RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Drop stale function that references listings and domain_id
DROP FUNCTION IF EXISTS get_listings_by_domain_page(UUID, TEXT);
