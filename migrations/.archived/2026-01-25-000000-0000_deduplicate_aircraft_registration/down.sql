-- Rollback: Restore non-unique index
-- NOTE: This does NOT restore the merged/deleted FLARM records or the nulled registrations
-- A full rollback would require restoring from a database backup

-- Drop unique index
DROP INDEX IF EXISTS idx_aircraft_registration_unique;

-- Recreate non-unique index
CREATE INDEX idx_aircraft_registration ON aircraft (registration);

-- NOTE: Registrations will remain NULL until the next data load
-- The merged FLARM records cannot be restored by this migration
