-- Rollback: Recreate non-unique index
-- Note: Cannot restore deleted aircraft or original registrations

DROP INDEX IF EXISTS idx_aircraft_registration_unique;

CREATE INDEX idx_aircraft_registration ON aircraft (registration);
