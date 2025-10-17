-- First, make device_id nullable in fixes table to allow CASCADE delete to set null
ALTER TABLE fixes ALTER COLUMN device_id DROP NOT NULL;

-- Delete all devices that were not from the DDB (spontaneously created devices)
-- This will also delete all fixes and flights associated with these devices via CASCADE
DELETE FROM devices WHERE from_ddb = FALSE;

-- Delete any orphaned fixes with NULL device_id (shouldn't happen but clean up just in case)
DELETE FROM fixes WHERE device_id IS NULL;

-- Restore NOT NULL constraint on device_id
ALTER TABLE fixes ALTER COLUMN device_id SET NOT NULL;

-- Add unique constraint on address column
-- This ensures that each device address is unique in the system
CREATE UNIQUE INDEX idx_devices_address_unique ON devices(address);
