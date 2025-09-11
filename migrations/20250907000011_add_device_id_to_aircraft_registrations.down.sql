-- Remove device_id column and index from aircraft_registrations table

DROP INDEX IF EXISTS aircraft_registrations_device_id_idx;
ALTER TABLE aircraft_registrations DROP COLUMN IF EXISTS device_id;