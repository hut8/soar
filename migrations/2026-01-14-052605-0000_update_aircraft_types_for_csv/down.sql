-- Revert migration: Update aircraft_types table for CSV data source

-- Drop composite primary key
ALTER TABLE aircraft_types DROP CONSTRAINT aircraft_types_pkey;

-- Remove NOT NULL constraint and default from iata_code
ALTER TABLE aircraft_types ALTER COLUMN iata_code DROP DEFAULT;
ALTER TABLE aircraft_types ALTER COLUMN iata_code DROP NOT NULL;

-- Restore empty strings back to NULL
UPDATE aircraft_types SET iata_code = NULL WHERE iata_code = '';

-- Restore original primary key (icao_code only)
ALTER TABLE aircraft_types ADD PRIMARY KEY (icao_code);

-- Drop new columns
ALTER TABLE aircraft_types
    DROP COLUMN manufacturer,
    DROP COLUMN wing_type,
    DROP COLUMN aircraft_category;

-- Drop enums
DROP TYPE icao_aircraft_category;
DROP TYPE wing_type;
