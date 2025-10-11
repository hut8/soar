-- Add location columns to flights table
ALTER TABLE flights ADD COLUMN takeoff_location_id UUID REFERENCES locations(id);
ALTER TABLE flights ADD COLUMN landing_location_id UUID REFERENCES locations(id);

-- Add indexes for the new foreign keys
CREATE INDEX idx_flights_takeoff_location_id ON flights(takeoff_location_id);
CREATE INDEX idx_flights_landing_location_id ON flights(landing_location_id);
