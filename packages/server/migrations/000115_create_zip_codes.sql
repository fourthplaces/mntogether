-- Reference table for zip code lat/lng lookups and proximity search
CREATE TABLE zip_codes (
    zip_code TEXT PRIMARY KEY,
    city TEXT NOT NULL,
    state TEXT NOT NULL DEFAULT 'MN',
    latitude DOUBLE PRECISION NOT NULL,
    longitude DOUBLE PRECISION NOT NULL
);

CREATE INDEX idx_zip_codes_state ON zip_codes(state);
CREATE INDEX idx_zip_codes_city_state ON zip_codes(city, state);

-- Haversine great-circle distance in miles
CREATE OR REPLACE FUNCTION haversine_distance(
    lat1 DOUBLE PRECISION, lng1 DOUBLE PRECISION,
    lat2 DOUBLE PRECISION, lng2 DOUBLE PRECISION
) RETURNS DOUBLE PRECISION AS $$
    SELECT 3959.0 * acos(
        LEAST(1.0, GREATEST(-1.0,
            cos(radians(lat1)) * cos(radians(lat2)) *
            cos(radians(lng2) - radians(lng1)) +
            sin(radians(lat1)) * sin(radians(lat2))
        ))
    )
$$ LANGUAGE sql IMMUTABLE STRICT;

-- Drop redundant service_areas table (tags with kind='service_area' handle this)
DROP TABLE IF EXISTS service_areas;
