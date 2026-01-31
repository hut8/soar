-- Replace the single (address_type, address) pair with four typed nullable address columns.
-- A single aircraft row can now hold multiple device addresses (e.g., both ICAO and Flarm).

-- Add four nullable address columns
ALTER TABLE aircraft ADD COLUMN icao_address INT4;
ALTER TABLE aircraft ADD COLUMN flarm_address INT4;
ALTER TABLE aircraft ADD COLUMN ogn_address INT4;
ALTER TABLE aircraft ADD COLUMN other_address INT4;

-- Populate from existing data (skip address = 0 as it's not a valid device address)
UPDATE aircraft SET icao_address = address WHERE address_type = 'icao' AND address != 0;
UPDATE aircraft SET flarm_address = address WHERE address_type = 'flarm' AND address != 0;
UPDATE aircraft SET ogn_address = address WHERE address_type = 'ogn' AND address != 0;
UPDATE aircraft SET other_address = address WHERE address_type = 'unknown' AND address != 0;

-- Delete aircraft with address = 0 (invalid placeholder data) and all dependent rows.
-- The fixes table is a TimescaleDB hypertable with compressed chunks. Deleting from
-- compressed chunks requires decompressing them first, which is expensive and slow.
-- Instead, drop the FK constraints from fixes, delete everything else, then re-add
-- the FKs. The orphaned fix rows will be cleaned up by the retention policy.

-- Drop FK constraints from fixes to avoid touching compressed chunks
ALTER TABLE fixes DROP CONSTRAINT fixes_aircraft_id_fkey;
ALTER TABLE fixes DROP CONSTRAINT fixes_flight_id_fkey;

-- Clear towed_by references pointing to affected aircraft/flights (nullable FKs)
UPDATE flights SET towed_by_aircraft_id = NULL
  WHERE towed_by_aircraft_id IN (SELECT id FROM aircraft WHERE address = 0);
UPDATE flights SET towed_by_flight_id = NULL
  WHERE towed_by_flight_id IN (SELECT id FROM flights WHERE aircraft_id IN (SELECT id FROM aircraft WHERE address = 0));

-- Delete dependent rows in FK order (fixes skipped -- FKs already dropped)
DELETE FROM flight_pilots WHERE flight_id IN (
  SELECT id FROM flights WHERE aircraft_id IN (SELECT id FROM aircraft WHERE address = 0)
);
DELETE FROM geofence_exit_events WHERE aircraft_id IN (SELECT id FROM aircraft WHERE address = 0);
DELETE FROM flights WHERE aircraft_id IN (SELECT id FROM aircraft WHERE address = 0);
DELETE FROM aircraft_geofences WHERE aircraft_id IN (SELECT id FROM aircraft WHERE address = 0);
DELETE FROM aircraft_registrations WHERE aircraft_id IN (SELECT id FROM aircraft WHERE address = 0);
DELETE FROM watchlist WHERE aircraft_id IN (SELECT id FROM aircraft WHERE address = 0);
DELETE FROM aircraft WHERE address = 0;

-- Re-add FK constraints on fixes with NOT VALID to skip scanning compressed chunks.
-- New inserts will still be validated; orphaned rows from deleted aircraft will be
-- cleaned up by the retention policy.
ALTER TABLE fixes ADD CONSTRAINT fixes_aircraft_id_fkey
  FOREIGN KEY (aircraft_id) REFERENCES aircraft(id) NOT VALID;
ALTER TABLE fixes ADD CONSTRAINT fixes_flight_id_fkey
  FOREIGN KEY (flight_id) REFERENCES flights(id) NOT VALID;

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
