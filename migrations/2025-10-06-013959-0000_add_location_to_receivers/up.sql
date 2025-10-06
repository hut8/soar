-- Add latitude, longitude, and location columns to receivers table
ALTER TABLE receivers
    ADD COLUMN latitude DOUBLE PRECISION,
    ADD COLUMN longitude DOUBLE PRECISION,
    ADD COLUMN location geography(Point, 4326)
        GENERATED ALWAYS AS (
            CASE
                WHEN latitude IS NOT NULL AND longitude IS NOT NULL
                THEN ST_Point(longitude, latitude)::geography
                ELSE NULL
            END
        ) STORED;

-- Create spatial index for efficient bounding box queries
CREATE INDEX idx_receivers_location ON receivers USING GIST (location);
