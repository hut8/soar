-- Add down migration script here
-- =========================================================
-- Revert aircraft_registrations table updates
-- =========================================================

-- Drop indexes
DROP INDEX IF EXISTS aircraft_registrations_registered_location_idx;
DROP INDEX IF EXISTS aircraft_registrations_home_base_airport_id_idx;

-- Drop columns
ALTER TABLE aircraft_registrations DROP COLUMN IF EXISTS registered_location;
ALTER TABLE aircraft_registrations DROP COLUMN IF EXISTS home_base_airport_id;