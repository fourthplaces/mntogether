-- The business_listings table was created in 000032 but never used
-- in practice. The Rust code referenced a "business_posts" table that
-- was never created by any migration. Remove the unused table.
DROP TABLE IF EXISTS business_listings;
