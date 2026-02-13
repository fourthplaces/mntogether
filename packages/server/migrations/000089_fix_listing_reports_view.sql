-- Fix the listing_reports_with_details view to use post_type instead of listing_type
-- This fixes the view after renaming listing_type to post_type

DROP VIEW IF EXISTS listing_reports_with_details;

CREATE OR REPLACE VIEW listing_reports_with_details AS
SELECT
  r.id,
  r.listing_id,
  r.reason,
  r.category,
  r.status,
  r.created_at,
  r.resolved_at,
  r.resolution_notes,
  r.action_taken,
  l.title as listing_title,
  l.organization_name,
  l.post_type,  -- Changed from listing_type
  l.status as listing_status,
  COUNT(*) OVER (PARTITION BY r.listing_id) as report_count_for_listing
FROM listing_reports r
INNER JOIN posts l ON l.id = r.listing_id  -- Changed from listings to posts
ORDER BY r.created_at DESC;
