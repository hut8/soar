-- Recreate location_geog columns on aircraft and user_fixes tables

-- Recreate on aircraft table
ALTER TABLE aircraft
    ADD COLUMN location_geog geography(Point, 4326)
        GENERATED ALWAYS AS (
            CASE
                WHEN latitude IS NOT NULL AND longitude IS NOT NULL
                THEN ST_Point(longitude, latitude)::geography
                ELSE NULL
            END
        ) STORED;

CREATE INDEX idx_aircraft_location_geog ON aircraft USING GIST (location_geog)
    WHERE location_geog IS NOT NULL;

-- Recreate on user_fixes table
ALTER TABLE user_fixes
    ADD COLUMN location_geog geography(Point, 4326)
        GENERATED ALWAYS AS (
            ST_Point(longitude, latitude)::geography
        ) STORED;

CREATE INDEX idx_user_fixes_location_geog ON user_fixes USING GIST (location_geog);
