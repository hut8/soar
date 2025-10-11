-- Restore the latitude, longitude, and location columns to receivers
-- This undoes the removal performed in up.sql

-- Re-add latitude and longitude columns
ALTER TABLE receivers
ADD COLUMN latitude DOUBLE PRECISION,
ADD COLUMN longitude DOUBLE PRECISION;

-- Re-add the generated location column
ALTER TABLE receivers
ADD COLUMN location geography(Point, 4326)
    GENERATED ALWAYS AS (
        CASE
            WHEN latitude IS NOT NULL AND longitude IS NOT NULL
            THEN ST_Point(longitude, latitude)::geography
            ELSE NULL
        END
    ) STORED;

-- Recreate spatial index for efficient bounding box queries
CREATE INDEX idx_receivers_location ON receivers USING GIST (location);

-- Restore the latitude/longitude data from locations table
UPDATE receivers r
SET
    latitude = l.geolocation[1],
    longitude = l.geolocation[0]
FROM locations l
WHERE r.location_id = l.id
  AND l.geolocation IS NOT NULL;
