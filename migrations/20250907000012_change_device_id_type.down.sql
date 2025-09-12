-- Reverse the device_id type change and column rename
-- Restore original foreign key relationships

-- Step 1: Drop foreign key constraints and columns from dependent tables
ALTER TABLE fixes DROP CONSTRAINT IF EXISTS fixes_device_id_fkey;
ALTER TABLE fixes DROP COLUMN IF EXISTS device_id;
ALTER TABLE aircraft_registrations DROP CONSTRAINT IF EXISTS aircraft_registrations_device_id_fkey;
DROP INDEX IF EXISTS aircraft_registrations_device_id_idx;
ALTER TABLE aircraft_registrations DROP COLUMN IF EXISTS device_id;

-- Step 2: Remove 3-byte constraint, change back to INTEGER, and rename id back to device_id in devices table
ALTER TABLE devices DROP CONSTRAINT IF EXISTS devices_id_3bytes_check;
ALTER TABLE devices ALTER COLUMN id TYPE INTEGER USING NULL;
ALTER TABLE devices RENAME COLUMN id TO device_id;

-- Step 3: Recreate device_id columns in dependent tables (back to original INTEGER without constraint)
ALTER TABLE fixes ADD COLUMN device_id INTEGER;
ALTER TABLE aircraft_registrations ADD COLUMN device_id INTEGER;

-- Step 4: Add foreign key constraints back (referencing device_id again)
ALTER TABLE aircraft_registrations ADD CONSTRAINT aircraft_registrations_device_id_fkey FOREIGN KEY (device_id) REFERENCES devices(device_id);

-- Step 5: Recreate indexes
CREATE INDEX aircraft_registrations_device_id_idx ON aircraft_registrations (device_id);
