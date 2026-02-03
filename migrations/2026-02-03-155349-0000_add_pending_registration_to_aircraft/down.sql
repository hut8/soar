DROP INDEX IF EXISTS idx_aircraft_pending_registration;
ALTER TABLE aircraft DROP COLUMN IF EXISTS pending_registration;
