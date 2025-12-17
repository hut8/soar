-- Add cached location columns to aircraft table
ALTER TABLE aircraft
    ADD COLUMN latitude DOUBLE PRECISION,
    ADD COLUMN longitude DOUBLE PRECISION,
    ADD COLUMN location_geom geometry(Point, 4326)
        GENERATED ALWAYS AS (
            CASE
                WHEN latitude IS NOT NULL AND longitude IS NOT NULL
                THEN ST_SetSRID(ST_MakePoint(longitude, latitude), 4326)
                ELSE NULL
            END
        ) STORED,
    ADD COLUMN location_geog geography(Point, 4326)
        GENERATED ALWAYS AS (
            CASE
                WHEN latitude IS NOT NULL AND longitude IS NOT NULL
                THEN ST_Point(longitude, latitude)::geography
                ELSE NULL
            END
        ) STORED;

-- Create indexes for spatial queries
CREATE INDEX idx_aircraft_location_geom ON aircraft USING GIST (location_geom)
    WHERE location_geom IS NOT NULL;

CREATE INDEX idx_aircraft_location_geog ON aircraft USING GIST (location_geog)
    WHERE location_geog IS NOT NULL;

-- Create index on latitude/longitude for non-spatial queries
CREATE INDEX idx_aircraft_lat_lon ON aircraft (latitude, longitude)
    WHERE latitude IS NOT NULL AND longitude IS NOT NULL;
