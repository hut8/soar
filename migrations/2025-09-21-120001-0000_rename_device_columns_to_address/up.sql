-- Rename device columns to address
-- 1. Rename device_id to address
-- 2. Rename device_id_type to address_type
-- 3. Rename DeviceType enum to AddressType

-- Step 1: Rename the PostgreSQL enum type
ALTER TYPE device_type RENAME TO address_type;

-- Step 2: Rename device_id column to address
ALTER TABLE devices RENAME COLUMN device_id TO address;

-- Step 3: Rename device_id_type column to address_type
ALTER TABLE devices RENAME COLUMN device_id_type TO address_type;

-- Step 4: Update the unique constraint to use the new column names
ALTER TABLE devices DROP CONSTRAINT IF EXISTS devices_device_id_type_device_id_unique;
ALTER TABLE devices ADD CONSTRAINT devices_address_type_address_unique UNIQUE (address_type, address);
