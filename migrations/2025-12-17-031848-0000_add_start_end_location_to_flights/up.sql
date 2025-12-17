-- Add start and end location columns to flights table for reverse geocoded addresses
-- start_location_id: Address where flight started (airport or airborne detection point)
-- end_location_id: Address where flight ended (airport or timeout point)

ALTER TABLE flights ADD COLUMN start_location_id UUID REFERENCES locations(id);
ALTER TABLE flights ADD COLUMN end_location_id UUID REFERENCES locations(id);

-- Add indexes for the new foreign keys
CREATE INDEX idx_flights_start_location_id ON flights(start_location_id);
CREATE INDEX idx_flights_end_location_id ON flights(end_location_id);
