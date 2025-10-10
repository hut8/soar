-- Add location_id column to receivers table
ALTER TABLE receivers
ADD COLUMN location_id UUID REFERENCES locations(id) ON DELETE SET NULL;

-- Create index on location_id for faster joins
CREATE INDEX idx_receivers_location_id ON receivers(location_id);
