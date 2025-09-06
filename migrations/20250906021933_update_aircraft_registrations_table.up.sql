-- Add up migration script here
-- =========================================================
-- Update aircraft_registrations table with location fields
-- =========================================================

-- Add home_base_airport_id as foreign key to airports table
ALTER TABLE aircraft_registrations ADD COLUMN home_base_airport_id UUID REFERENCES airports(id) ON DELETE SET NULL;

-- Add registered_location as WGS84 point (where the aircraft is registered)
ALTER TABLE aircraft_registrations ADD COLUMN registered_location POINT;

-- Create index on home_base_airport_id
CREATE INDEX aircraft_registrations_home_base_airport_id_idx ON aircraft_registrations (home_base_airport_id);

-- Create spatial index on registered_location
CREATE INDEX aircraft_registrations_registered_location_idx ON aircraft_registrations USING GIST (registered_location);