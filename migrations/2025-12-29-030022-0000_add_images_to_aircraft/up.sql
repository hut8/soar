-- Add images JSONB column to aircraft table for caching fetched images
ALTER TABLE aircraft
ADD COLUMN IF NOT EXISTS images JSONB;

COMMENT ON COLUMN aircraft.images IS 'Cached aircraft images from external sources (airport-data.com, etc.)';

-- Create GIN index for JSONB queries (optional but recommended for performance)
CREATE INDEX idx_aircraft_images ON aircraft USING GIN (images);
