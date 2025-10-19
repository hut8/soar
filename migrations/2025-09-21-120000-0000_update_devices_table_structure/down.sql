-- Revert devices table structure changes
-- This reverses all changes made in the up migration

-- Step 1: Drop new foreign key constraints
ALTER TABLE aircraft_registrations DROP CONSTRAINT IF EXISTS aircraft_registrations_device_id_fkey;
ALTER TABLE fixes DROP CONSTRAINT IF EXISTS fixes_device_id_fkey;

-- Step 2: Add old device_id columns back to referencing tables
ALTER TABLE aircraft_registrations ADD COLUMN old_device_id INT4;
ALTER TABLE fixes ADD COLUMN old_device_id INT4;

-- Step 3: Populate old device_id columns from devices table
UPDATE aircraft_registrations
SET old_device_id = devices.device_id
FROM devices
WHERE aircraft_registrations.device_id = devices.id;

UPDATE fixes
SET old_device_id = devices.device_id
FROM devices
WHERE fixes.device_id = devices.id;

-- Step 4: Drop new UUID device_id columns
ALTER TABLE aircraft_registrations DROP COLUMN device_id;
ALTER TABLE fixes DROP COLUMN device_id;

-- Step 5: Rename old columns back to device_id
ALTER TABLE aircraft_registrations RENAME COLUMN old_device_id TO device_id;
ALTER TABLE fixes RENAME COLUMN old_device_id TO device_id;

-- Step 6: Update devices table - drop UUID primary key and constraints
ALTER TABLE devices DROP CONSTRAINT devices_pkey;
ALTER TABLE devices DROP CONSTRAINT IF EXISTS devices_device_id_type_device_id_unique;

-- Step 7: Rename device_id_type back to device_type
ALTER TABLE devices RENAME COLUMN device_id_type TO device_type;

-- Step 8: Add old primary key back on device_id
ALTER TABLE devices ADD PRIMARY KEY (device_id);

-- Step 9: Drop UUID column
ALTER TABLE devices DROP COLUMN id;

-- Step 10: Restore old foreign key constraints
ALTER TABLE aircraft_registrations
ADD CONSTRAINT aircraft_registrations_device_id_fkey
FOREIGN KEY (device_id) REFERENCES devices(device_id) ON DELETE SET NULL;

ALTER TABLE fixes
ADD CONSTRAINT fixes_device_id_fkey
FOREIGN KEY (device_id) REFERENCES devices(device_id) ON DELETE SET NULL;
