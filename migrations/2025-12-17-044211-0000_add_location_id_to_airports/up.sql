-- Add location_id to airports table to store reverse-geocoded address
ALTER TABLE airports ADD COLUMN location_id UUID REFERENCES locations(id);

-- Add index for the foreign key
CREATE INDEX idx_airports_location_id ON airports(location_id);
