-- Reverse: restore the single (address_type, address) pair from typed address columns.

-- Re-add the old columns
ALTER TABLE aircraft ADD COLUMN address INT4;
ALTER TABLE aircraft ADD COLUMN address_type address_type NOT NULL DEFAULT 'unknown';

-- Populate from typed columns (priority: icao > flarm > ogn > other)
UPDATE aircraft SET address = icao_address, address_type = 'icao' WHERE icao_address IS NOT NULL;
UPDATE aircraft SET address = flarm_address, address_type = 'flarm' WHERE flarm_address IS NOT NULL AND address IS NULL;
UPDATE aircraft SET address = ogn_address, address_type = 'ogn' WHERE ogn_address IS NOT NULL AND address IS NULL;
UPDATE aircraft SET address = other_address, address_type = 'unknown' WHERE other_address IS NOT NULL AND address IS NULL;

-- Set default for any remaining rows
UPDATE aircraft SET address = 0 WHERE address IS NULL;
ALTER TABLE aircraft ALTER COLUMN address SET NOT NULL;

-- Re-add old unique constraint
ALTER TABLE aircraft ADD CONSTRAINT aircraft_address_type_address_unique UNIQUE (address_type, address);

-- Drop new columns and constraints
ALTER TABLE aircraft DROP CONSTRAINT chk_at_least_one_address;
DROP INDEX idx_aircraft_icao_address;
DROP INDEX idx_aircraft_flarm_address;
DROP INDEX idx_aircraft_ogn_address;
DROP INDEX idx_aircraft_other_address;
ALTER TABLE aircraft DROP COLUMN icao_address;
ALTER TABLE aircraft DROP COLUMN flarm_address;
ALTER TABLE aircraft DROP COLUMN ogn_address;
ALTER TABLE aircraft DROP COLUMN other_address;
