-- Move club_id from aircraft_registrations to devices table
-- First, copy club_id values from aircraft_registrations to devices where they match on device_id
UPDATE devices
SET club_id = aircraft_registrations.club_id
FROM aircraft_registrations
WHERE devices.id = aircraft_registrations.device_id
  AND aircraft_registrations.club_id IS NOT NULL;

-- Drop the club_id column from aircraft_registrations
ALTER TABLE aircraft_registrations DROP COLUMN club_id;
