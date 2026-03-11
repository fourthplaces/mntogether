-- service_listings and opportunity_listings were created in 000032 as
-- type-specific extension tables for the old "listings" entity.
-- The listings table was later renamed to posts, but these extension
-- tables were never migrated. No Rust code references them.
DROP TABLE IF EXISTS service_listings;
DROP TABLE IF EXISTS opportunity_listings;
DROP TABLE IF EXISTS listing_delivery_modes;
DROP TABLE IF EXISTS listing_contacts;
