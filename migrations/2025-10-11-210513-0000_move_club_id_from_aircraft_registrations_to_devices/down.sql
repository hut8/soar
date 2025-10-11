-- Reverse: Move club_id back from devices to aircraft_registrations

-- Add club_id column back to aircraft_registrations
ALTER TABLE aircraft_registrations ADD COLUMN club_id UUID;

-- Re-create the foreign key constraint to clubs
ALTER TABLE aircraft_registrations
    ADD CONSTRAINT aircraft_registrations_club_id_fkey
    FOREIGN KEY (club_id)
    REFERENCES clubs(id);

-- Copy club_id values back from devices to aircraft_registrations where they match on device_id
UPDATE aircraft_registrations
SET club_id = devices.club_id
FROM devices
WHERE aircraft_registrations.device_id = devices.id
  AND devices.club_id IS NOT NULL;
