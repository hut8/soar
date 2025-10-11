-- Migrate existing receiver location data (latitude/longitude) to the locations table
-- and link receivers to those locations via location_id

-- Insert location records for receivers that have latitude/longitude but no location_id yet
INSERT INTO locations (id, geolocation, created_at, updated_at)
SELECT
    gen_random_uuid(),
    point(r.longitude, r.latitude),
    now(),
    now()
FROM receivers r
WHERE r.latitude IS NOT NULL
  AND r.longitude IS NOT NULL
  AND r.location_id IS NULL
ON CONFLICT DO NOTHING;

-- Update receivers to link to the newly created location records
-- We need to find the matching location by geolocation
UPDATE receivers r
SET location_id = l.id
FROM locations l
WHERE r.latitude IS NOT NULL
  AND r.longitude IS NOT NULL
  AND r.location_id IS NULL
  AND l.geolocation = point(r.longitude, r.latitude);
