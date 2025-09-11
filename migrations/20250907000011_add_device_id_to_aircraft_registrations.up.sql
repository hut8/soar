-- Add device_id column to aircraft_registrations table
-- This links aircraft registrations to their corresponding devices

ALTER TABLE aircraft_registrations 
ADD COLUMN device_id INTEGER REFERENCES devices(device_id);

-- Create index for efficient device lookups
CREATE INDEX aircraft_registrations_device_id_idx ON aircraft_registrations (device_id);