-- Add down migration script here
-- =========================================================
-- Revert locations table creation and normalization
-- =========================================================

-- 1. Drop indexes first
DROP INDEX IF EXISTS clubs_location_id_idx;
DROP INDEX IF EXISTS aircraft_registrations_location_id_idx;

-- 2. Remove location_id foreign key column from clubs
ALTER TABLE clubs DROP COLUMN IF EXISTS location_id;

-- 3. Remove location_id foreign key column from aircraft_registrations  
ALTER TABLE aircraft_registrations DROP COLUMN IF EXISTS location_id;

-- 4. Drop the locations table (this will cascade and remove all references)
DROP TABLE IF EXISTS locations;