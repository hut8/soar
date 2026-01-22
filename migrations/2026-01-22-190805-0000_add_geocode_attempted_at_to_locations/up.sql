-- Add column to track when geocoding was last attempted
-- This allows us to distinguish between "never attempted" (NULL) and "attempted but failed" (non-NULL with geolocation still NULL)
ALTER TABLE locations ADD COLUMN geocode_attempted_at TIMESTAMPTZ;

-- Create index for efficient querying of locations needing geocoding
CREATE INDEX idx_locations_needs_geocoding
    ON locations (geocode_attempted_at)
    WHERE geolocation IS NULL AND geocode_attempted_at IS NULL;
