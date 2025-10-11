-- Migrate existing receiver location data (latitude/longitude) to the locations table
-- and link receivers to those locations via location_id

-- Temporarily drop the unique constraint on address fields to allow multiple geolocation-only records
DROP INDEX IF EXISTS locations_address_unique_idx;

-- Insert unique location records for each distinct receiver coordinate pair
INSERT INTO locations (id, geolocation, created_at, updated_at)
SELECT
    gen_random_uuid(),
    point(longitude, latitude),
    now(),
    now()
FROM (
    SELECT DISTINCT longitude, latitude
    FROM receivers
    WHERE latitude IS NOT NULL
      AND longitude IS NOT NULL
      AND location_id IS NULL
) AS distinct_coords
WHERE NOT EXISTS (
    SELECT 1 FROM locations l
    WHERE l.geolocation[0] = distinct_coords.longitude
      AND l.geolocation[1] = distinct_coords.latitude
);

-- Update receivers to link to their matching location records
UPDATE receivers r
SET location_id = l.id
FROM locations l
WHERE r.latitude IS NOT NULL
  AND r.longitude IS NOT NULL
  AND r.location_id IS NULL
  AND l.geolocation[0] = r.longitude
  AND l.geolocation[1] = r.latitude;

-- Recreate the unique constraint, but only for records with actual address data
-- This allows multiple geolocation-only records while still preventing duplicate addresses
CREATE UNIQUE INDEX locations_address_unique_idx ON locations (
    COALESCE(street1, ''),
    COALESCE(street2, ''),
    COALESCE(city, ''),
    COALESCE(state, ''),
    COALESCE(zip_code, ''),
    COALESCE(country_mail_code, 'US')
)
WHERE street1 IS NOT NULL OR city IS NOT NULL OR zip_code IS NOT NULL;
