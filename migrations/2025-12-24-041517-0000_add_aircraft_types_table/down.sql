-- Drop indexes
DROP INDEX IF EXISTS idx_aircraft_types_description;
DROP INDEX IF EXISTS idx_aircraft_types_iata_code;

-- Drop table
DROP TABLE IF EXISTS aircraft_types;
