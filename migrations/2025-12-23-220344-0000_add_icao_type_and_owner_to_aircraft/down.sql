-- Remove ICAO aircraft type code and owner/operator columns from aircraft table

DROP INDEX IF EXISTS idx_aircraft_icao_type_code;
DROP INDEX IF EXISTS idx_aircraft_owner_operator;

ALTER TABLE aircraft
DROP COLUMN IF EXISTS icao_type_code,
DROP COLUMN IF EXISTS owner_operator;
