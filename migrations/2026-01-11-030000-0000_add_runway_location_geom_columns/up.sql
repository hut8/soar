-- Add generated geometry columns for faster spatial queries (avoids runtime geography->geometry cast)
ALTER TABLE runways
    ADD COLUMN le_location_geom geometry(Point, 4326) GENERATED ALWAYS AS (le_location::geometry) STORED,
    ADD COLUMN he_location_geom geometry(Point, 4326) GENERATED ALWAYS AS (he_location::geometry) STORED;

-- Create spatial indexes on the geometry columns for fast bounding box queries
CREATE INDEX idx_runways_le_location_geom ON runways USING GIST (le_location_geom);
CREATE INDEX idx_runways_he_location_geom ON runways USING GIST (he_location_geom);
