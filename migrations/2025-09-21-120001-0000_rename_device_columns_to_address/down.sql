-- Revert device column renames back to original names
-- 1. Rename address back to device_id
-- 2. Rename address_type back to device_id_type
-- 3. Rename AddressType enum back to DeviceType

-- Step 1: Update the unique constraint to use the old column names
ALTER TABLE devices DROP CONSTRAINT IF EXISTS devices_address_type_address_unique;
ALTER TABLE devices ADD CONSTRAINT devices_device_id_type_device_id_unique UNIQUE (device_id_type, device_id);

-- Step 2: Rename address_type column back to device_id_type
ALTER TABLE devices RENAME COLUMN address_type TO device_id_type;

-- Step 3: Rename address column back to device_id
ALTER TABLE devices RENAME COLUMN address TO device_id;

-- Step 4: Rename the PostgreSQL enum type back to device_type
ALTER TYPE address_type RENAME TO device_type;
