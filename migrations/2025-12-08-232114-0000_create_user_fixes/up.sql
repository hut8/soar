-- Create user_fixes table for tracking user locations
CREATE TABLE user_fixes (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    latitude DOUBLE PRECISION NOT NULL,
    longitude DOUBLE PRECISION NOT NULL,
    heading DOUBLE PRECISION, -- Compass direction phone is pointing (0-360 degrees, null if unavailable)
    location_geom geometry(Point, 4326)
        GENERATED ALWAYS AS (
            ST_SetSRID(ST_MakePoint(longitude, latitude), 4326)
        ) STORED,
    location_geog geography(Point, 4326)
        GENERATED ALWAYS AS (
            ST_Point(longitude, latitude)::geography
        ) STORED,
    raw JSONB, -- Additional sensor data (accuracy, altitude, speed, orientation, etc.)
    timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Create indexes for efficient queries
CREATE INDEX idx_user_fixes_user_id ON user_fixes (user_id);
CREATE INDEX idx_user_fixes_user_timestamp ON user_fixes (user_id, timestamp DESC);
CREATE INDEX idx_user_fixes_location_geom ON user_fixes USING GIST (location_geom);
CREATE INDEX idx_user_fixes_location_geog ON user_fixes USING GIST (location_geog);
