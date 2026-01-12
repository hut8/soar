-- Drop the old trigger-based geography columns and their indexes
DROP TRIGGER IF EXISTS update_runway_locations_trigger ON runways;
DROP FUNCTION IF EXISTS public.update_runway_locations();
DROP INDEX IF EXISTS idx_runways_le_location_gist;
DROP INDEX IF EXISTS idx_runways_he_location_gist;
ALTER TABLE runways DROP COLUMN IF EXISTS le_location;
ALTER TABLE runways DROP COLUMN IF EXISTS he_location;

-- Add generated geometry columns directly from lat/lon for fast spatial queries
ALTER TABLE runways
    ADD COLUMN le_location_geom geometry(Point, 4326)
        GENERATED ALWAYS AS (
            CASE WHEN le_latitude_deg IS NOT NULL AND le_longitude_deg IS NOT NULL
            THEN ST_SetSRID(ST_MakePoint(le_longitude_deg::float8, le_latitude_deg::float8), 4326)
            END
        ) STORED,
    ADD COLUMN he_location_geom geometry(Point, 4326)
        GENERATED ALWAYS AS (
            CASE WHEN he_latitude_deg IS NOT NULL AND he_longitude_deg IS NOT NULL
            THEN ST_SetSRID(ST_MakePoint(he_longitude_deg::float8, he_latitude_deg::float8), 4326)
            END
        ) STORED;

-- Create spatial indexes on the geometry columns for fast bounding box queries
CREATE INDEX idx_runways_le_location_geom ON runways USING GIST (le_location_geom) WHERE le_location_geom IS NOT NULL;
CREATE INDEX idx_runways_he_location_geom ON runways USING GIST (he_location_geom) WHERE he_location_geom IS NOT NULL;
