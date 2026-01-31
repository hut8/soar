-- Replace the single (address_type, address) pair with four typed nullable address columns.
-- A single aircraft row can now hold multiple device addresses (e.g., both ICAO and Flarm).

-- Add four nullable address columns
ALTER TABLE aircraft ADD COLUMN icao_address INT4;
ALTER TABLE aircraft ADD COLUMN flarm_address INT4;
ALTER TABLE aircraft ADD COLUMN ogn_address INT4;
ALTER TABLE aircraft ADD COLUMN other_address INT4;

-- Populate from existing data
UPDATE aircraft SET icao_address = address WHERE address_type = 'icao';
UPDATE aircraft SET flarm_address = address WHERE address_type = 'flarm';
UPDATE aircraft SET ogn_address = address WHERE address_type = 'ogn';
UPDATE aircraft SET other_address = address WHERE address_type = 'unknown' AND address != 0;

-- Add unique indexes (each address type has its own unique namespace).
-- Plain unique indexes (not partial) because PostgreSQL treats NULLs as distinct,
-- and Diesel's on_conflict() cannot infer partial unique indexes.
CREATE UNIQUE INDEX idx_aircraft_icao_address ON aircraft(icao_address);
CREATE UNIQUE INDEX idx_aircraft_flarm_address ON aircraft(flarm_address);
CREATE UNIQUE INDEX idx_aircraft_ogn_address ON aircraft(ogn_address);
CREATE UNIQUE INDEX idx_aircraft_other_address ON aircraft(other_address);

-- Ensure at least one address is set
ALTER TABLE aircraft ADD CONSTRAINT chk_at_least_one_address
  CHECK (icao_address IS NOT NULL OR flarm_address IS NOT NULL OR ogn_address IS NOT NULL OR other_address IS NOT NULL);

-- Drop old columns and constraints
ALTER TABLE aircraft DROP CONSTRAINT aircraft_address_type_address_unique;
ALTER TABLE aircraft DROP COLUMN address;
ALTER TABLE aircraft DROP COLUMN address_type;
-- Note: Do NOT drop the address_type enum type; the flights table still uses it.
