-- Add index on device_id to improve lookup performance
CREATE INDEX IF NOT EXISTS idx_aircraft_registrations_device_id
ON aircraft_registrations(device_id);
