-- Update devices table structure
-- 1. Add new UUID primary key column
-- 2. Rename device_type to device_id_type
-- 3. Remove old primary key constraint
-- 4. Add new primary key on UUID
-- 5. Add unique constraint on (device_id_type, device_id)
-- 6. Update foreign key references in other tables

-- Step 1: Add UUID extension if not exists
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- Step 2: Add new UUID primary key column to devices table
ALTER TABLE devices ADD COLUMN id UUID DEFAULT uuid_generate_v4() NOT NULL;

-- Step 3: Rename device_type column to device_id_type
ALTER TABLE devices RENAME COLUMN device_type TO device_id_type;

-- Step 4: Drop existing foreign key constraints first
ALTER TABLE aircraft_registrations DROP CONSTRAINT IF EXISTS aircraft_registrations_device_id_fkey;
ALTER TABLE fixes DROP CONSTRAINT IF EXISTS fixes_device_id_fkey;

-- Step 5: Add new device_id columns to tables that reference devices
ALTER TABLE aircraft_registrations ADD COLUMN new_device_id UUID;
ALTER TABLE fixes ADD COLUMN new_device_id UUID;

-- Step 6: Populate new device_id columns with UUIDs from devices table
UPDATE aircraft_registrations
SET new_device_id = devices.id
FROM devices
WHERE aircraft_registrations.device_id = devices.device_id;

UPDATE fixes
SET new_device_id = devices.id
FROM devices
WHERE fixes.device_id = devices.device_id;

-- Step 7: Drop old device_id columns
ALTER TABLE aircraft_registrations DROP COLUMN device_id;
ALTER TABLE fixes DROP COLUMN device_id;

-- Step 8: Rename new columns to device_id
ALTER TABLE aircraft_registrations RENAME COLUMN new_device_id TO device_id;
ALTER TABLE fixes RENAME COLUMN new_device_id TO device_id;

-- Step 9: Update devices table - drop old primary key and add new one
ALTER TABLE devices DROP CONSTRAINT devices_pkey;
ALTER TABLE devices ADD PRIMARY KEY (id);

-- Step 10: Add unique constraint on (device_id_type, device_id)
ALTER TABLE devices ADD CONSTRAINT devices_device_id_type_device_id_unique UNIQUE (device_id_type, device_id);

-- Step 11: Add new foreign key constraints
ALTER TABLE aircraft_registrations
ADD CONSTRAINT aircraft_registrations_device_id_fkey
FOREIGN KEY (device_id) REFERENCES devices(id) ON DELETE SET NULL;

ALTER TABLE fixes
ADD CONSTRAINT fixes_device_id_fkey
FOREIGN KEY (device_id) REFERENCES devices(id) ON DELETE SET NULL;