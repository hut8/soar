-- Change device_id to be exactly 3 bytes (24 bits) and rename to "id"
-- Update foreign key relationships in fixes and aircraft_registrations tables

-- Step 1: Drop foreign key constraints and columns from dependent tables
ALTER TABLE fixes DROP COLUMN IF EXISTS device_id;
ALTER TABLE aircraft_registrations DROP CONSTRAINT IF EXISTS aircraft_registrations_device_id_fkey;
DROP INDEX IF EXISTS aircraft_registrations_device_id_idx;
ALTER TABLE aircraft_registrations DROP COLUMN IF EXISTS device_id;

-- Step 2: Rename device_id to id in devices table and change to BYTEA with 3-byte constraint
ALTER TABLE devices RENAME COLUMN device_id TO id;
ALTER TABLE devices ALTER COLUMN id TYPE BIT(24) USING NULL;

-- Step 3: Recreate device_id columns in dependent tables with BYTEA type and 3-byte constraint
ALTER TABLE fixes ADD COLUMN device_id BIT(24);
ALTER TABLE aircraft_registrations ADD COLUMN device_id BIT(24);

-- Step 4: Add foreign key constraints back
ALTER TABLE fixes ADD CONSTRAINT fixes_device_id_fkey FOREIGN KEY (device_id) REFERENCES devices(id);
ALTER TABLE aircraft_registrations ADD CONSTRAINT aircraft_registrations_device_id_fkey FOREIGN KEY (device_id) REFERENCES devices(id);

-- Step 5: Recreate indexes
CREATE INDEX aircraft_registrations_device_id_idx ON aircraft_registrations (device_id);
