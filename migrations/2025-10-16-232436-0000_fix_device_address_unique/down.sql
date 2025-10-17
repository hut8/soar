-- Remove unique constraint on address column
DROP INDEX IF EXISTS idx_devices_address_unique;

-- Delete any orphaned fixes with NULL device_id before restoring NOT NULL constraint
DELETE FROM fixes WHERE device_id IS NULL;

-- Make device_id NOT NULL again in fixes table
ALTER TABLE fixes ALTER COLUMN device_id SET NOT NULL;

-- Note: We cannot restore the deleted devices that had from_ddb = FALSE
-- This migration is not fully reversible
